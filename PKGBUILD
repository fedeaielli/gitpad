# Maintainer: https://github.com/fedeaielli
# The v$pkgver tag must exist on the remote.
pkgname=gitpad
pkgver=1.0.5
pkgrel=1
pkgdesc="Markdown notes in a Git repository you own"
arch=('x86_64')
url="https://github.com/fedeaielli/gitpad"
license=('GPL-3.0-only')
depends=('webkit2gtk-4.1' 'gtk3' 'libsoup3' 'openssl' 'libsecret' 'libgit2' 'hicolor-icon-theme')
makedepends=('rust' 'nodejs' 'npm' 'git')
# The tag carries this file, so use the tag as a VCS source: an archive cannot
# contain the checksum of itself.
source=("$pkgname::git+$url.git#tag=v$pkgver")
sha256sums=('SKIP')
# ring ships assembly that LTO turns into bitcode ld cannot resolve.
options=('!lto')

prepare() {
  cd "$pkgname"
  npm ci
  (cd src-tauri && cargo fetch --locked)
}

build() {
  cd "$pkgname"
  npm run build
  # Public OAuth client id: the device flow must be compiled in, like the
  # AppImage build does via the `npm run tauri` script.
  export GITHUB_CLIENT_ID=Ov23lilamnIFNDISE1uQ
  # Tauri only serves the bundled assets with this feature on: without it the
  # binary thinks it is a dev build and loads devUrl (http://localhost:1420).
  (cd src-tauri && cargo build --release --locked --offline --features tauri/custom-protocol)
}

package() {
  cd "$pkgname"
  install -Dm755 src-tauri/target/release/gitpad "$pkgdir/usr/bin/gitpad"
  install -Dm644 gitpad.desktop "$pkgdir/usr/share/applications/gitpad.desktop"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  install -Dm644 src-tauri/icons/32x32.png \
    "$pkgdir/usr/share/icons/hicolor/32x32/apps/gitpad.png"
  install -Dm644 src-tauri/icons/64x64.png \
    "$pkgdir/usr/share/icons/hicolor/64x64/apps/gitpad.png"
  install -Dm644 src-tauri/icons/128x128.png \
    "$pkgdir/usr/share/icons/hicolor/128x128/apps/gitpad.png"
  install -Dm644 src-tauri/icons/128x128@2x.png \
    "$pkgdir/usr/share/icons/hicolor/256x256/apps/gitpad.png"
}
