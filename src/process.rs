//! Work-around for <https://github.com/zellij-org/zellij/issues/896>
use once_cell::sync::OnceCell;
use std::{
    fmt::Write as _,
    fs::File,
    io::prelude::*,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};
use zellij_tile::prelude::*;

pub fn spawn_and_get_output(cmd: &[u8]) -> Vec<u8> {
    let proxy = PROXY.get_or_init(|| Mutex::new(Proxy::new()));
    proxy.lock().unwrap().spawn_and_get_output(cmd)
}

static PROXY: OnceCell<Mutex<Proxy>> = OnceCell::new();

struct Proxy {
    pipe_req: File,
    pipe_res: File,
    pipe_buf_path: PathBuf,
}

/// The shell command to start [`PROXY_SCRIPT`].
///
/// Variable `tmp` contains the value of `ZELLIJ_TMP_DIR`
/// (`zellij-utils/src/consts.rs`).
const PROXY_LOADER: &str = "\
    tmp=\"/tmp/zellij-`id -u`\"; \
    mv $tmp/$1 $tmp/$2; \
    exec /bin/sh $tmp/$2 $3";

const PROXY_SCRIPT: &str = r#"
    set -eux
    pipe="$1"

    tmp="`dirname "$0"`"
    trap 'rm -f $tmp/$pipe-*' exit
    mkfifo "$tmp/$pipe-res" "$tmp/$pipe-req"

    while read -r line; do
        # TODO: writing to a file on disk every time might cause wear on SSD
        eval "$line" > "$tmp/$pipe-buf"
        printf '!' >&3
    done < "$tmp/$pipe-req" 3> "$tmp/$pipe-res"
"#;

impl Proxy {
    fn new() -> Self {
        let mut pipe_name = concat!(env!("CARGO_PKG_NAME"), "-pipe-").to_owned();
        {
            let mut buf = [0u8; 16];
            getrandom::getrandom(&mut buf).expect("failed to generate random numbers");
            for b in buf.iter() {
                write!(pipe_name, "{b:02x}").unwrap();
            }
        }

        // ZELLIJ_TMP_DIR is mapped here from the plugin VM point of view
        // (see `zellij-server/src/wasm_vm.rs`)
        let tmp_root = "/tmp";
        let pipe_req_path = format!("{tmp_root}/{pipe_name}-req");
        let pipe_res_path = format!("{tmp_root}/{pipe_name}-res");
        let pipe_buf_path = PathBuf::from(format!("/tmp/{pipe_name}-buf"));

        let script_name = concat!(
            env!("CARGO_PKG_NAME"),
            "-",
            env!("CARGO_PKG_VERSION"),
            "-subproc"
        );
        let script_name_tmp = format!("{script_name}+{pipe_name}");

        // Generate the proxy script
        let script_path_tmp = format!("{tmp_root}/{script_name_tmp}");
        std::fs::write(&script_path_tmp, PROXY_SCRIPT)
            .unwrap_or_else(|e| panic!("failed to write '{script_path_tmp}': {e:?}"));

        // Execute the proxy script
        exec_cmd(&[
            "/bin/sh",
            "-c",
            PROXY_LOADER,
            "subproc",
            &script_name_tmp,
            &script_name,
            &pipe_name,
        ]);

        // Open the pipes
        let timeout = Duration::from_secs(5);
        let pipe_req =
            retry_until_success(|| File::options().write(true).open(&pipe_req_path), timeout)
                .unwrap_or_else(|e| panic!("failed to open '{pipe_req_path}': {e:?}"));
        let pipe_res =
            retry_until_success(|| File::options().read(true).open(&pipe_res_path), timeout)
                .unwrap_or_else(|e| panic!("failed to open '{pipe_res_path}': {e:?}"));

        Self {
            pipe_req,
            pipe_res,
            pipe_buf_path,
        }
    }

    fn spawn_and_get_output(&mut self, cmd: &[u8]) -> Vec<u8> {
        assert!(!cmd.contains(&b'\n'));
        self.pipe_req
            .write_all(cmd)
            .expect("failed to send a request to a subprocess proxy");
        self.pipe_req
            .write_all(b"\n")
            .expect("failed to send a request to a subprocess proxy");

        let mut buf = [0u8; 1];
        self.pipe_res
            .read_exact(&mut buf)
            .expect("failed to read a response from a subprocess proxy");

        std::fs::read(&self.pipe_buf_path).unwrap_or_else(|e| {
            panic!(
                "failed to read a command output from '{}': {e:?}",
                self.pipe_buf_path.display()
            )
        })
    }
}

fn retry_until_success<R, E>(
    mut f: impl FnMut() -> Result<R, E>,
    timeout: Duration,
) -> Result<R, E> {
    let mut result;
    let start = Instant::now();
    while {
        result = f();
        result.is_err() && start.elapsed() < timeout
    } {}
    result
}
