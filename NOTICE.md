# Third-party notices

The top-level [MIT License](LICENSE) covers CC-Guard's original work. It does not replace third-party copyright notices or license terms.

## CCBaga

CC-Guard is derived from [liseami/CCBaga](https://github.com/liseami/CCBaga), released under the MIT License (Copyright (c) 2026 CCBaga contributors). The identity vault, browser and Windows platform code originate there. That copyright notice is retained in [LICENSE](LICENSE). CCBaga's Claude data discovery was informed by the [claude-cleanup skill](https://github.com/suyuan2022/suyuan-skill/tree/main/claude-cleanup).

## Fingerprint Chromium and Chromium

The bundled web-version browser uses [Fingerprint Chromium](https://github.com/adryfish/fingerprint-chromium), based on Ungoogled Chromium. Its BSD-3-Clause notice is retained in [browser-runtime/LICENSE.fingerprint-chromium](browser-runtime/LICENSE.fingerprint-chromium) and shipped with the application. The engine contains Chromium and further dependencies under their own licenses; this single notice is not a complete inventory. The engine is downloaded at build time with a pinned SHA-256 and is not stored in this repository.

## Windows timezone data

Windows/IANA mappings derive from [Unicode CLDR 48](https://github.com/unicode-org/cldr/blob/release-48/common/supplemental/windowsZones.xml) under the [Unicode License](src-tauri/windows/LICENSE.unicode.txt), which ships with the Windows package.

## Application dependencies

Tauri, React, Radix UI, Tailwind CSS, lucide-react, Vite, YAML and the Rust crates keep their own licenses. `bun.lock` and `src-tauri/Cargo.lock` record the versions used.

## Trademarks

Claude and Anthropic are trademarks of Anthropic. Clash, mihomo, Windows, Chromium and other names belong to their owners. Mentioning them does not imply sponsorship or endorsement.
