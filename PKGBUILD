# Maintainer: https://github.com/fedeaielli
# The v$pkgver tag must exist on the remote.
pkgname=gitpad
pkgver=0.1.5
pkgrel=1
pkgdesc="Markdown notes in a Git repository you own"
arch=('x86_64')
url="https://github.com/fedeaielli/gitpad"
license=('GPL-3.0-only')
depends=('webkit2gtk-4.1' 'gtk3' 'libsoup3' 'openssl' 'libsecret' 'libgit2' 'hicolor-icon-theme')
makedepends=('rust' 'nodejs' 'npm')
source=("$pkgname-$pkgver.tar.gz::$url/archive/v$pkgver.tar.gz")
sha256sums=('da4a4f0c579e254ed772acafe1bfeb09a70577a619f0a3229dd0309746eccd67')

prepare() {
  cd "$pkgname-$pkgver"
  npm ci
  (cd src-tauri && cargo fetch --locked)
}

build() {
  cd "$pkgname-$pkgver"
  npm run build
  (cd src-tauri && cargo build --release --locked --offline)
}

package() {
  cd "$pkgname-$pkgver"
  install -Dm755 src-tauri/target/release/gitpad "$pkgdir/usr/bin/gitpad"
  install -Dm644 gitpad.desktop "$pkgdir/usr/share/applications/gitpad.desktop"
  install -Dm644 src-tauri/icons/32x32.png \
    "$pkgdir/usr/share/icons/hicolor/32x32/apps/gitpad.png"
  install -Dm644 src-tauri/icons/64x64.png \
    "$pkgdir/usr/share/icons/hicolor/64x64/apps/gitpad.png"
  install -Dm644 src-tauri/icons/128x128.png \
    "$pkgdir/usr/share/icons/hicolor/128x128/apps/gitpad.png"
  install -Dm644 src-tauri/icons/128x128@2x.png \
    "$pkgdir/usr/share/icons/hicolor/256x256/apps/gitpad.png"
}
