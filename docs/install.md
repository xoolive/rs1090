# Installation instructions

The latest release and installation instructions are published [on GitHub Releases](https://github.com/xoolive/jet1090/releases/).

Current version is v0.4.1

## Install prebuilt binaries

=== "Shell script"

    This script requires that you install the [SoapySDR dependencies](#dependencies).

    ```sh
    curl --proto '=https' --tlsv1.2 -LsSf https://github.com/xoolive/jet1090/releases/download/v0.4.14/jet1090-installer.sh | sh
    ```

    Update to next release with:

    ```sh
    jet1090-update
    ```

=== "Powershell (Windows)"

    This script requires that you install the [SoapySDR dependencies](#dependencies).

    ```sh
    powershell -ExecutionPolicy ByPass -c "irm https://github.com/xoolive/jet1090/releases/download/v0.4.14/jet1090-installer.ps1 | iex"
    ```

    Update to next release with:

    ```sh
    jet1090-update
    ```

=== "Homebrew"

    SoapySDR dependencies are automatically installed.

    ```sh
    brew install xoolive/homebrew/jet1090
    ```

    Update to next release with:

    ```sh
    brew upgrade
    ```

## Dependencies

The prebuilt binaries are compiled with all features activated. In particular, support for RTL-SDR is provided through SoapySDR which may require extra dependencies.

=== "Ubuntu"

    ```sh
    sudo apt install libsoapysdr-dev soapysdr-module-rtlsdr
    ```

=== "Arch Linux"

    ```sh
    sudo pacman -S soapyrtlsdr
    ```

=== "Homebrew"

    Dependencies are automatically installed if you install `jet1090` through Homebrew. You have to run the command to build the project from source.

    ```sh
    brew install soapysdr soapyrtlsdr
    ```

=== "Windows"

    - Install [PothosSDR](https://downloads.myriadrf.org/builds/PothosSDR/PothosSDR-2021.07.25-vc16-x64.exe). If you don't have admin rights, you may unzip the archive and add the `bin/` folder to your `PATH` variable.
    - You will also need [Zadig](https://zadig.akeo.ie/) to install the drivers for your RTL-SDR dongle (admin rights necessary).

## Build from source

=== "cargo"

    You will need:

    - the SoapySDR dependencies to compile with the `rtlsdr` feature.
    - a protobuf compiler to compile with the `sero` feature.

    ```sh
    cargo install --all-features jet1090
    ```

    Note that a protobuf compiler is also necessary to compile the project with the `sero` feature:

    === "Ubuntu"

        ```sh
        sudo apt install protobuf-compiler
        ```

    === "Homebrew"

        ```sh
        brew install protobuf
        ```

    === "Windows"

        - Install [Protobuf compiler](https://github.com/protocolbuffers/protobuf/releases/download/v28.3/protoc-28.3-win64.zip)

=== "nix"

    Nix takes care of its own dependencies. The script has been tested for Linux and MacOS.

    ```sh
    git clone https://github.com/xoolive/jet1090
    nix profile install
    ```

## Shell completion

=== "Bash"

    Add the following to the end of your `~/.bashrc`:

    ```sh
    eval "$(jet1090 --completion bash)"
    ```

=== "Zsh"

    Add the following to the end of your `~/.zshrc`:

    ```sh
    eval "$(jet1090 --completion zsh)"
    ```

=== "fish"

    Add the following to the end of `~/.config/fish/config.fish`:

    ```fish
    jet1090 --completion fish | source
    ```

=== "Powershell"

    Add the following to the end of `Microsoft.PowerShell_profile.ps1`. You can check the location of this file by querying the `$PROFILE` variable in PowerShell. Typically the path is` ~\Documents\PowerShell\Microsoft.PowerShell_profile.ps1` or `~/.config/powershell/Microsoft.PowerShell_profile.ps1` on -Nix.

    ```powershell
    (& jet1090 --completion powershell) | Out-String | Invoke-Expression
    ```

=== "Nushell"

    Add the following to the end of your Nushell env file (find it by running `$nu.env-path`):

    ```sh
    mkdir -p ~/.config/jet1090
    jet1090 --completion nushell | save -f ~/.config/jet1090/completions.nu
    ```

    then, add the following to the end of your Nushell configuration (find it by running `$nu.config-path`):

    ```sh
    use ~/.config/jet1090/completions.nu *
    ```

=== "Elvish"

    Add the following to the end of `~/.elvish/rc.elv`:

    ```sh
    eval (jet1090 --completion elvish | slurp)
    ```
