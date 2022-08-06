<div align="center">

# zellij-cpulamp

**Minimal CPU activity indicator plugin for [Zellij][1]**

![](https://yvt.jp/files/programs/zellij-cpulamp/2022-08-07-screenshot-3.gif)

</div>

Displays blinking dots representing each processor's usage.

# Usage

## From source

```shell
cargo build --release
zellij --layout example-layout.yaml
```

**note:** [The plugin interface][2] is unstable. You may need to edit
`Cargo.toml` and change the version of `zellij-tile` to get the plugin working.

## License

This program is licensed under the GNU Lesser General Public License version 3
or later.

[1]: https://zellij.dev/
[2]: https://zellij.dev/documentation/plugins.html
