#!/usr/bin/env sh
release_binary="habasi"
release_folder="Habasi"
release_zip="${release_folder}.zip"
osxcross_path="${HOME}/projects/osxcross/target/bin" # only used for macos builds

build() {
  set -x
  cargo build --target x86_64-unknown-linux-gnu --profile release-lto || return 1
  cargo build --target x86_64-unknown-linux-musl --profile release-lto || return 1
  cargo ndk --target arm64-v8a build --profile release-lto || return 1
  cargo xwin build --target x86_64-pc-windows-msvc --profile release-lto || return 1
  cargo +1.77.2 xwin build --target x86_64-pc-windows-msvc --profile release-lto --target-dir "target/1.77.2_win7" || return 1
  # cargo build --profile release-lto --target x86_64-pc-windows-gnu || return 1
  PATH="${osxcross_path}:$PATH" cargo build --target x86_64-apple-darwin --profile release-lto-darwin\
    --config target.x86_64-apple-darwin.linker=\"x86_64-apple-darwin21.4-clang\"\
    --config target.x86_64-apple-darwin.ar=\"x86_64-apple-darwin21.4-ar\" || return 1
  PATH="${osxcross_path}:$PATH" cargo build --target aarch64-apple-darwin --profile release-lto-darwin\
    --config target.aarch64-apple-darwin.linker=\"aarch64-apple-darwin21.4-clang\"\
    --config target.aarch64-apple-darwin.ar=\"aarch64-apple-darwin21.4-ar\" || return 1
  set +x
}

zip() (
  if [ -d "${release_folder}" ]; then
    echo "${release_folder} exists"
    return 1
  fi
  if [ -f "${release_zip}" ]; then
    rm -v "${release_zip}" || return 1
  fi
  mkdir -pv "${release_folder}/linux_x86-64" || return 1
  cp    -vt "${release_folder}/linux_x86-64"\
    "target/x86_64-unknown-linux-gnu/release-lto/${release_binary}" || return 1
  mkdir -pv "${release_folder}/linux_x86-64_musl" || return 1
  cp    -vt "${release_folder}/linux_x86-64_musl"\
    "target/x86_64-unknown-linux-musl/release-lto/${release_binary}" || return 1
  mkdir -pv "${release_folder}/android_aarch64" || return 1
  cp    -vt "${release_folder}/android_aarch64"\
    "target/aarch64-linux-android/release-lto/${release_binary}" || return 1
  mkdir -pv "${release_folder}/windows_x86-64" || return 1
  cp    -vt "${release_folder}/windows_x86-64"\
    "target/x86_64-pc-windows-msvc/release-lto/${release_binary}.exe" || return 1
  mkdir -pv "${release_folder}/windows_x86-64_win7" || return 1
  cp    -vt "${release_folder}/windows_x86-64_win7"\
    "target/1.77.2_win7/x86_64-pc-windows-msvc/release-lto/${release_binary}.exe" || return 1
  # mkdir -pv "${release_folder}/windows_x86-64_gnu" || return 1
  # cp    -vt "${release_folder}/windows_x86-64_gnu"\
  #   "target/x86_64-pc-windows-gnu/release-lto/${release_binary}.exe" || return 1
  mkdir -pv "${release_folder}/macos_x86-64" || return 1
  cp    -vt "${release_folder}/macos_x86-64"\
    "target/x86_64-apple-darwin/release-lto-darwin/${release_binary}" || return 1
  mkdir -pv "${release_folder}/macos_aarch64" || return 1
  cp    -vt "${release_folder}/macos_aarch64"\
    "target/aarch64-apple-darwin/release-lto-darwin/${release_binary}" || return 1
  cp   -rvt "${release_folder}/"\
    "_configuration_examples" || return 1

  7z a "${release_zip}" "${release_folder}" -tzip || return 1
  7z t "${release_zip}" || return 1
  7z l "${release_zip}" || return 1
  md5sum "${release_zip}" || return 1
  rm -r "${release_folder}" || return 1
)

main() {
  cargo clippy --all -- -D clippy::all -D clippy::pedantic -D warnings -A clippy::blanket-clippy-restriction-lints || return 1
  cargo msrv verify || return 1
  build || return 1
  if [ "${1}" = "zip" ]; then
    zip || return 1
  fi
}

main "$@" || echo "error"

# [Build for your platform]
#
# RUSTFLAGS="-C target-cpu=native" cargo build --profile release-lto

# [Preparations on arch-linux to build for other platforms]
#
# rustup target add x86_64-unknown-linux-gnu x86_64-unknown-linux-musl x86_64-pc-windows-gnu x86_64-pc-windows-gnu x86_64-apple-darwin aarch64-apple-darwin
# [Preparations:android]
# yay -S android-ndk cargo-ndk
# [Preparations:windows_MSVC]
# cargo install cargo-xwin
# [Preparations:windows_GNU]
# yay -S mingw-w64-gcc
# [Preparations:windows_WIN7]
# rustup install 1.77.2
# rustup +1.77.2 target add x86_64-pc-windows-msvc
# rustup +1.77.2 show
# [Preparations:macOS]
# yay -S clang
# # https://wapl.es/rust/2019/02/17/rust-cross-compile-linux-to-macos.html
# # https://github.com/tpoechtrager/osxcross - how to get sdk
# # go to mac, install homebrew with Xcode Command Line Tools
# git clone https://github.com/tpoechtrager/osxcross.git
# cd osxcross/
# ./tools/gen_sdk_package_tools.sh
# # transfer files to linux
# git clone https://github.com/tpoechtrager/osxcross.git
# cd osxcross/
# cp ../MacOSX12.3.sdk.tar.xz tarballs/
# ./build.sh
# [Preparations:msrv]
# yay -S cargo-msrv

# [PGO template] Doesn't improve anything for this project.
#
# rustup component add llvm-tools-preview
# rm -rf /tmp/pgo-data/
# RUSTFLAGS="-C target-cpu=native -C profile-generate=/tmp/pgo-data" cargo build --profile release-lto
# ./target/release/habasi
# ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata merge -o /tmp/pgo-data/merged.profdata /tmp/pgo-data
# RUSTFLAGS="-C target-cpu=native -C profile-use=/tmp/pgo-data/merged.profdata" cargo build --profile release-lto
