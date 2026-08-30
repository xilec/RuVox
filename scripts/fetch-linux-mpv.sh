#!/usr/bin/env bash
# Downloads and assembles the pinned mpv player bundle that the AppImage
# ships (issue #265): the Ubuntu noble /usr/bin/mpv binary plus its
# non-core shared-library closure, flattened into
# src-tauri/resources/mpv-linux/ with RPATH=$ORIGIN on every file, so the
# spawned player resolves its libraries from its own directory —
# tauri-plugin-mpv starts it with an inherited environment and offers no
# LD_LIBRARY_PATH knob. The .deb does NOT carry this resource (it Depends
# on the system mpv instead, #266); the resource is wired into AppImage
# builds only, via src-tauri/tauri.appimage.conf.json.
#
# The manifest below was captured from a stock ubuntu:24.04 container
# (matching the CI runner and the local Docker builder) by walking the
# DT_NEEDED closure of /usr/bin/mpv and mapping every non-core library
# back to its exact noble .deb (glibc family, libstdc++, libgcc_s excluded
# — AppImage core libs every host provides; Ubuntu hardening -z now makes
# the full loader closure mandatory). Each file is downloaded from the
# launchpad primary archive — it keeps every published version, so the
# pins are stable; the epoch prefix "N%3a" is stripped for the URL — and
# sha256-verified: any mismatch or failed download aborts the build.
#
# Re-pinning (newer mpv or a moved noble point release): inside a stock
# ubuntu:24.04 container, `apt-get install -y --no-install-recommends mpv`,
# walk the ldd closure of /usr/bin/mpv to a fixpoint, map each non-core
# library back with `dpkg -S "$(readlink -f …)"`, `apt-get download` every
# package and record (filename, sha256, kind) — kind=BIN is the package
# owning /usr/bin/mpv, kind=LIB contribute only their
# /usr/lib/x86_64-linux-gnu/*.so* (top level plus the blas/, lapack/ and
# pulseaudio/ subdirs: the first two hold update-alternatives-managed
# sonames that only materialize after dpkg's postinst, the last holds
# libpulse's private libpulsecommon). Replace the MANIFEST block and run a
# full build; the loader sweep at the end gates the result.
#
# Runs on the Linux CI runner before `tauri build` (release.yml
# linux-packages job) and inside scripts/build-linux-packages.sh; needs
# curl, dpkg-deb, patchelf and ldd. The committed README-only placeholder
# tree keeps compile-time resource validation green between releases —
# never bundle a build where it is still a placeholder (`--check` enforces
# this for release-style builds).

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="$repo_root/src-tauri/resources/mpv-linux"

if [ "${1:-}" = "--check" ]; then
    # Release-build guard: a real ELF player must be present, not the
    # placeholder tree (mirrors the "never bundle placeholders" rule).
    [ -s "$target/mpv" ] || { echo "error: $target/mpv is missing or a placeholder — run $0 first" >&2; exit 1; }
    head -c 4 "$target/mpv" | grep -q $'\x7fELF' || { echo "error: $target/mpv is not an ELF executable" >&2; exit 1; }
    exit 0
fi

for tool in curl dpkg-deb patchelf ldd; do
    command -v "$tool" >/dev/null || { echo "error: $tool is required" >&2; exit 1; }
done

MANIFEST="
libacl1_2.3.2-1build1.1_amd64.deb	f2bfd3f8f00413d5f1f04fc723063803c56ac0f1e0efae3bc41f2d7276972ec3	LIB
libaom3_3.8.2-2ubuntu0.1_amd64.deb	1623e49454a2981069ed4801c853630a63a68f77c73056dea11e3289a8467eb9	LIB
libapparmor1_4.0.1really4.0.1-0ubuntu0.24.04.7_amd64.deb	4205351c37f4e813f1ca81b6d59a00071f0f70869e652f4ab9e5ba7e5e895d34	LIB
libarchive13t64_3.7.2-2ubuntu0.8_amd64.deb	ba9684092d71656a3bfd909518c76cc15eefeef9cbac0a36a8cef5632c62f56f	LIB
libasound2t64_1.2.11-1ubuntu0.3_amd64.deb	5d17bfb5683eb99f1be951fe48e1be5bf3c52391913b6b74a8aecf9b4f5e779f	LIB
libass9_1%3a0.17.1-2build1_amd64.deb	a3dd7b4e6fa05d0d6607739b3c1f60a305f2fb78d9f6e8b8e38d10d0b4fc4052	LIB
libasyncns0_0.8-6build4_amd64.deb	f8483917fd216e248143eba1c80dfc9384b94fb4dac402194a98c11505674269	LIB
libavc1394-0_0.5.4-5build3_amd64.deb	28141f0e1a2011deef364a931b4a0fb9ab053416320ed7fd0b8747f251e6e5d7	LIB
libavcodec60_7%3a6.1.1-3ubuntu5_amd64.deb	970d92c1697f762235af439d72df8c6cb492be45050a1a00bbe0d0842a137f93	LIB
libavdevice60_7%3a6.1.1-3ubuntu5_amd64.deb	62c55a1209332142eb01488894b3443060e59b44940189469ac7f8fbe03406ca	LIB
libavfilter9_7%3a6.1.1-3ubuntu5_amd64.deb	083b800aaf4fd1de817fb8dc12995e6a277c3500159c4a0fa21d36a76b6208ea	LIB
libavformat60_7%3a6.1.1-3ubuntu5_amd64.deb	3a855488d50b4ebe04425e5f690e00ebd65d70e60f85c9765d5823a2f82b4969	LIB
libavutil58_7%3a6.1.1-3ubuntu5_amd64.deb	e57f8cc358f4b1b2af721ca6242723dc12290df543a142811ce7747e2d5b30cc	LIB
libblas3_3.12.0-3build1.1_amd64.deb	1b0ef0ad0893ff77bf7d5ab75723771788e16137ea1efde557dbac7b7b83a758	LIB
libblkid1_2.39.3-9ubuntu6.5_amd64.deb	2307a9ed92642a69de0284ac63e233e45ac5a2c3831f04399f7d212379487e82	LIB
libbluray2_1%3a1.3.4-1build1_amd64.deb	2cfb2a7e4b6170efcfb96e0dbe3e9321b648acc43883ab4e7c248216de18ea6f	LIB
libbrotli1_1.1.0-2build2_amd64.deb	74492419b8fda803774b8c9acef6afc5d2f9ff31782635aae212906adae7b277	LIB
libbs2b0_3.1.0+dfsg-7build1_amd64.deb	67d8af009d520e0b6f63e39d6d04d8c02e681f89858fb301b808a6c356068e2c	LIB
libbsd0_0.12.1-1build1.1_amd64.deb	f3857b0863ac5cfd4263e9bf6cfb1d4be88d5321e4070d5bc2b62b0949e6c86f	LIB
libbz2-1.0_1.0.8-5.1build0.1_amd64.deb	d557ab12b42ab370249142099fae3cbb979948934e4dfa58c2ab59bf5bbbda73	LIB
libcaca0_0.99.beta20-4ubuntu0.2_amd64.deb	f8ba81982beda87da75bef9a1035a2ff34479d613dfea757440442ae9d58c7af	LIB
libcairo-gobject2_1.18.0-3build1_amd64.deb	f5483cbe43455fc9e9cba7f0e166bd0a731c02db2902ba562719848f72e8c403	LIB
libcairo2_1.18.0-3build1_amd64.deb	96950b306889ff6e4248bb937b0a3b56e72dacf643fe732214ce375f0f6bbb36	LIB
libcap2_1%3a2.66-5ubuntu2.4_amd64.deb	23c627c77c66552a658e62852f244f4946dc543898e0ec5428d5dc6bffde2d6a	LIB
libcdio-cdda2t64_10.2+2.0.1-1.1build2_amd64.deb	9651b439c9f8134052602cf5c03c5216e826bc6782a07baf1745d4dac411b109	LIB
libcdio-paranoia2t64_10.2+2.0.1-1.1build2_amd64.deb	4ca9df983c50971bf42a60ffb9072faa4b6af785bd0e3592d3511f37cc540503	LIB
libcdio19t64_2.1.0-4.1ubuntu1.2_amd64.deb	afe010f4b8fd43751e774889347d9f271bb327f3ba4a402443e679e0a6ebe50d	LIB
libchromaprint1_1.5.1-5_amd64.deb	ff766f8382fff4e5d3b267e4cb93f31e1805ebbb9a619cc8c805b9a2355baeea	LIB
libcjson1_1.7.17-1_amd64.deb	43d758e214612877ec9029f2384bdbb0a6eeba6e74bb2cd3e7a60f4108947c41	LIB
libcodec2-1.2_1.2.0-2build1_amd64.deb	675a16e7260cc6773a325b9688c29f29fb7089f5780bc5360154ed6136d6331b	LIB
libcom-err2_1.47.0-2.4~exp1ubuntu4.1_amd64.deb	7ab24d3057dabf86db8f771ad6e43f073ed86b6b950d6e8ba22cb9fe6707bbc9	LIB
libdatrie1_0.2.13-3build1_amd64.deb	1ae164e40e413eac4fbd38ce1c6ef9591f7a03278ad9076d71fa29258727f447	LIB
libdav1d7_1.4.1-1build1_amd64.deb	7246303bd3ed4b99a2fb98e6199950ceb2520e7572265aefa0a6e44352adb3f0	LIB
libdb5.3t64_5.3.28+dfsg2-7_amd64.deb	a78a25c8fad8fdd0b7bc6b297da5d5685579be1e57732aa47870830e4a13161e	LIB
libdbus-1-3_1.14.10-4ubuntu4.1_amd64.deb	5d630480f04b4b442300ce847a3fa705ea4d14d80ba6de91f99b51a4e4953b08	LIB
libdc1394-25_2.2.6-4build1_amd64.deb	0f4a0768152636f2f941d3a29d24925be77f869b3d90a13f1694fef792a89810	LIB
libdecor-0-0_0.2.2-1build2_amd64.deb	64bf085d16e504e9ed39b91d2a46748fb8881ad7f3603fbacef6c11c04aa7064	LIB
libdrm2_2.4.125-1ubuntu0.1~24.04.2_amd64.deb	4d3e858dd57be617c49d87c5ddcdb35df744a4f0a559981256ae2ba174718584	LIB
libdvdnav4_6.1.1-3build1_amd64.deb	09fd1de2985ade2592d4f068774b2249211880aa6d4a71a89e0b4764afe47404	LIB
libdvdread8t64_6.1.3-1.1build1_amd64.deb	0d6af5aed25a51ccdb3d0fcad5a1e85cfe9c5aa86a895e313eab30157b76eb2d	LIB
libegl1_1.7.0-1build1_amd64.deb	e549f7776216f7bd3b1c216729fb40a97a562e2eb7c563cde632b4931f9a4e57	LIB
libexpat1_2.6.1-2ubuntu0.4_amd64.deb	126a5612e652bdc2edee19ae8fe4308db72b5b3b0a5581bf885b44a093baf3e5	LIB
libffi8_3.4.6-1build1_amd64.deb	637e6a7744de08cd331a41f4efd0d24e6ea9064843dea9d1c6ca87bdb5f038a2	LIB
libfftw3-double3_3.3.10-1ubuntu3_amd64.deb	ff9396076103b305efe741e9acff11d52a040c6d95527c0f2dbd11e82a11bab8	LIB
libflac12t64_1.4.3+ds-2.1ubuntu2_amd64.deb	f2396692573407b2fbc92cd83148342d4aec717d42d5e24ac64c471204a0b7a9	LIB
libflite1_2.2-6build3_amd64.deb	367f1d0da5cd38759a0515eafc27aa133b2d7bf99308cac34831df0212e96b75	LIB
libfontconfig1_2.15.0-1.1ubuntu2_amd64.deb	a2bc05cfef021fdb84285036f98eda5ebec3c4b7a378f5aa2bdba5c4d3d8d586	LIB
libfreetype6_2.13.2+dfsg-1ubuntu0.1_amd64.deb	f6937fd8a77e83001dcfd3857d5a5cfbd4caaeb297c7fdb71ec50514843e48af	LIB
libfribidi0_1.0.13-3build1_amd64.deb	6cd50259d39ce0dfafee2632c6268538e6a02590e77161a133e9683af346dd1d	LIB
libgbm1_25.2.8-0ubuntu0.24.04.2_amd64.deb	1c0b8daf0130d68de428334ce6d1e11bd11f7369bf9099bbc65ac88117336dc3	LIB
libgcrypt20_1.10.3-2ubuntu0.1_amd64.deb	69569eb6b4ab1de6dad556d06cabb236b9a8b07145edf18197256a0dedfaabc9	LIB
libgdk-pixbuf-2.0-0_2.42.10+dfsg-3ubuntu3.3_amd64.deb	de84e0d777d9c9e39148d8491338e6b8c62bc2d066867266b1643a7a725a86ba	LIB
libgfortran5_14.2.0-4ubuntu2~24.04.1_amd64.deb	2b05896c477275ad3473bb63a5ac5a5b8a7e4a6ad6ed7cb23c442ccd1abc28f5	LIB
libgl1_1.7.0-1build1_amd64.deb	e2b2fadeb883f073b566ad1d7874f6702397514d258473e96152a89d4d502a09	LIB
libglib2.0-0t64_2.80.0-6ubuntu3.8_amd64.deb	ca4eda0e01c76ba2e5e501e77decce53d98a641234b517eb98fc5d74980f913f	LIB
libglvnd0_1.7.0-1build1_amd64.deb	33f5e07c74f73c2bfb44086ae0f9e6da52acd6c104d30ffe8fd768d8253d5e82	LIB
libglx0_1.7.0-1build1_amd64.deb	aa4953182f30abde90cb5b072e83d7286b557b325769b8686196a7bd396d8795	LIB
libgme0_0.6.3-7build1_amd64.deb	a54fa608501f87f9e5064b60c75fc1bb62735c9f9ba9f7680cb804bb5eae2027	LIB
libgmp10_2%3a6.3.0+dfsg-2ubuntu6.1_amd64.deb	285f8a505dfa8e1b33f357a9d8d3477ad35bf18c0b34771a6df4c25923f3ae0d	LIB
libgnutls30t64_3.8.3-1.1ubuntu3.6_amd64.deb	b8944756260c5ea6b7fac019745f931f905aa7a4df0763676b1b25c911b692e2	LIB
libgomp1_14.2.0-4ubuntu2~24.04.1_amd64.deb	e8a95ec58125b4933597f30ff56c2ae10edf90f287262e366d4b6edea3019144	LIB
libgpg-error0_1.47-3build2.1_amd64.deb	93654ee8180a73a0363f25c51dc673d67cbabcbecd164187b8a2deb54d007aef	LIB
libgraphite2-3_1.3.14-2ubuntu0.24.04.1_amd64.deb	2413ea0f6670d6610ee7c2f550551d69fa341f4b9c78e20eb397f0d1bbe914cb	LIB
libgsm1_1.0.22-1build1_amd64.deb	8cc765587a3275e861f166a98b4b79dd8d5c673f97e17967945ab4d1eb89c89a	LIB
libgssapi-krb5-2_1.20.1-6ubuntu2.8_amd64.deb	6cd99ec16ae12eb465712f950e43eaf03a8d2a6ab24c00178df56470d5343b66	LIB
libharfbuzz0b_8.3.0-2build2_amd64.deb	d6f7eea2244f98aa0463a056680d4629476bf624767ede301560e24add686b5c	LIB
libhogweed6t64_3.9.1-2.2build1.1_amd64.deb	a9b5f7e9d49ba9060e1c933567046fbc6feab6096799cbf550b7214dc9b0f49b	LIB
libhwy1t64_1.0.7-8.1build1_amd64.deb	c092b1ea155d81a5985073f867e1479c48c5c6efe4adc1bfcdde27e5d2b8b89d	LIB
libicu74_74.2-1ubuntu3.1_amd64.deb	c9a70989678660eed9a1e904c74fa043da8bec8e2036856fc16e31ced79b04f8	LIB
libidn2-0_2.3.7-2build1.1_amd64.deb	46bfd10df095a23b65f58115de47a547c9a2d14627749bd0423ae78c14be77d3	LIB
libiec61883-0_1.2.0-6build1_amd64.deb	a47c6ec10068f1fc37a334949f6174f7ecbf0889bfde9d36baea8d403d76a727	LIB
libjack-jackd2-0_1.9.21~dfsg-3ubuntu3_amd64.deb	4fc5f4d0fbf2602df59bbed3ea504742bdcc7f899d7b32f6b2fd446ceb1fdf7d	LIB
libjpeg-turbo8_2.1.5-2ubuntu2_amd64.deb	f68b5b23bc8a1688fb787d2aed7e2cdf895a73022f6a5025e183162dac4500b2	LIB
libjxl0.7_0.7.0-10.2ubuntu6.1_amd64.deb	dc4a6436f7ab5e887c157d70a2784494be0db43b55fbb3a00d87d6385057f51d	LIB
libk5crypto3_1.20.1-6ubuntu2.8_amd64.deb	48f689737191cfafaf3c158e9b07d6448f9e6217ad7abbaacc4f96dc95403fa2	LIB
libkeyutils1_1.6.3-3build1_amd64.deb	0679f198b0128179e46cdf956fb2022c23c758664c00bc8efa0382d509683a8a	LIB
libkrb5-3_1.20.1-6ubuntu2.8_amd64.deb	63ab8110daea359f55d8135d395de198257acb1f948500c561745addddfece4c	LIB
libkrb5support0_1.20.1-6ubuntu2.8_amd64.deb	cee1efc93d4ce4a97db756269824b5a2b90d2cb993cd76102432db60890819fc	LIB
liblapack3_3.12.0-3build1.1_amd64.deb	05ced5049d4fd393e0ae6dc71211e7c07ed0c85e8099649d679cf4b5a1aed59b	LIB
liblcms2-2_2.14-2ubuntu0.1_amd64.deb	f48368ce55ac35c4723a158c74f2c48f4398570d6f3207874e3a14e1cdada71d	LIB
liblilv-0-0_0.24.22-1build1_amd64.deb	5fd6b6861a021e8148c9086d36b70eea28a503dd4c2f2b922f56789d2967d0ad	LIB
liblua5.2-0_5.2.4-3build2_amd64.deb	b030bc0bea32fe27dfb70a31e14491b6ab767379811877e6497af28338eab5d8	LIB
liblz4-1_1.9.4-1build1.1_amd64.deb	319331270d5cc52d5ebffe51c941d7b01b432bc402c2924b557209a64d4ecbad	LIB
liblzma5_5.6.1+really5.4.5-1ubuntu0.3_amd64.deb	d2eabd41ca77d2c2dd9d5d4ef478cccb64ffde6279c47cf4699a857d46785a52	LIB
libmbedcrypto7t64_2.28.8-1_amd64.deb	1f0b795a2274f68edddaf2567d738fcee43383c2c4ba802d54ba3b710bbb3228	LIB
libmd0_1.1.0-2build1.1_amd64.deb	e5ba01d3c41f256aaf57ec59aa0554857162e3e7f97cdfbff1ed2c0e8d720ee7	LIB
libmount1_2.39.3-9ubuntu6.5_amd64.deb	7ca31424fbfc96fbf245e6fe232fd2d2ec74169ca1add968e1987bece5bc0d1f	LIB
libmp3lame0_3.100-6build1_amd64.deb	d739e9a3565f98b8b72a9e203a9a537bd1090af6cafb94c98adf8e27f9d3fb94	LIB
libmpg123-0t64_1.32.5-1ubuntu1.1_amd64.deb	bbe32128e3358f141f9f58ef6e76a7bef3e07d09165203665d30fdc27372b418	LIB
libmujs3_1.3.3-3build2_amd64.deb	eb65d35b3cee83aeecaa4ccdbfbde2452d5bb394237f4668f1bb6cc154e61ff0	LIB
libmysofa1_1.3.2+dfsg-2ubuntu2_amd64.deb	a3b5bb4bcc6ce996700f0120722b1c10af08e9dd1e0d497782edcea90079d4af	LIB
libncursesw6_6.4+20240113-1ubuntu2.1_amd64.deb	bc860e63d1f8e653b1d14695ed5e3e8baf88b14c38ba0e6a93c67809a7c116a6	LIB
libnettle8t64_3.9.1-2.2build1.1_amd64.deb	6d97fbc1972633083f08f51ccab433606c97bbceb897c631c66495117ca3406f	LIB
libnorm1t64_1.5.9+dfsg-3.1build1_amd64.deb	0ff2047ae4ccf2d1b5f338922f98a8652344ee1c0f18dc80a419753d06582cc6	LIB
libnuma1_2.0.18-1ubuntu0.24.04.1_amd64.deb	f333b8edf6f0b705c19f2a67008194df66f7aab0fa2350dc98510569a033ff29	LIB
libogg0_1.3.5-3build1_amd64.deb	fbdb2fedbafb02c3f07f2c2ddeb4018b5e94e13462f93fbececdcefc06fe2ac1	LIB
libopenal1_1%3a1.23.1-4build1_amd64.deb	1781774a540ab7c9baccc41b49592e3b427c5820c64e390af4868d6a861ef8c7	LIB
libopenjp2-7_2.5.0-2ubuntu0.5_amd64.deb	24b92ce9c4b306f5370e768024d35ec565792001678b510f0d40b6395b43524f	LIB
libopenmpt0t64_0.7.3-1.1build3_amd64.deb	512814f1ebdf8fe31b8185826ba08f7933998b7a7ac1599fac39717dbaaae6ff	LIB
libopus0_1.4-1build1_amd64.deb	6d8da32a832e655c9603543b47a0bc90c6f60bdee0c00fe7499bd50acad566cf	LIB
libp11-kit0_0.25.3-4ubuntu2.2_amd64.deb	e7c58aeac19d89f5f1492dfaa52b8623bcd56edc40983f5243ad56d94f477bf2	LIB
libpango-1.0-0_1.52.1+ds-1build1_amd64.deb	09dfa5c881ab273ec6bc1830adcefdc407fcd619316e68fb26ca3a23d0fc9f54	LIB
libpangocairo-1.0-0_1.52.1+ds-1build1_amd64.deb	98cf5fc9076c2911fe20dfc1e41cd8f686a2bb5d539b1105fab18105b0db1376	LIB
libpangoft2-1.0-0_1.52.1+ds-1build1_amd64.deb	cc6bc9d86ef5f329edacaa4895232d8a9b8f95c5cbc77c3c460a3f64cecc213d	LIB
libpcre2-8-0_10.42-4ubuntu2.1_amd64.deb	110a797a57673d3ee497a141cf988199258058c57525799c63194d81822529a0	LIB
libpgm-5.3-0t64_5.3.128~dfsg-2.1build1_amd64.deb	9382625866a207b3da34f541799d10203a5693e5928c8d08f6d10be1c545d0a5	LIB
libpipewire-0.3-0t64_1.0.5-1ubuntu3.3_amd64.deb	225786f83ecb81075b820652038e20f38ae8f72a9787b766c4e0aa1bd7dfead8	LIB
libpixman-1-0_0.42.2-1build1_amd64.deb	d9c2931c4c424615eeab9dd5ae08bfc608b84e803c1d3ccddf319270db213421	LIB
libplacebo338_6.338.2-2build1_amd64.deb	9c173df229673887df3cce7d4a26de15f7477a62cd1391f9a88c346018ecad1f	LIB
libpng16-16t64_1.6.43-5ubuntu0.6_amd64.deb	ac0163f58b8aa52dfdd17cdc10a83f3db084cb91a62686023f5c983d6ff89f8b	LIB
libpocketsphinx3_0.8.0+real5prealpha+1-15ubuntu5_amd64.deb	26987384225666ec8d065fa05e484be9cd5967e28ac3bda20d1f818617e2b930	LIB
libpostproc57_7%3a6.1.1-3ubuntu5_amd64.deb	f0984bf6bff85dc29a44f650841b01a0594775f628b7a4bf5dd45d85d82cb2a7	LIB
libpulse0_1%3a16.1+dfsg1-2ubuntu10.1_amd64.deb	b8d52aa6c2f74ae99e749c9f98d6ad2dbf924fd422371ac469725245cdd919fc	LIB
librabbitmq4_0.11.0-1ubuntu0.1_amd64.deb	362ba3da304edaacd272e86aa9d88f855b6c8ed86316c15196207e4419a36a13	LIB
librav1e0_0.7.1-2_amd64.deb	be10d7e2dc028d9d7791997a8710b7a9a3147151613750f0f08963fbe9ac993c	LIB
libraw1394-11_2.1.2-2build3_amd64.deb	0a6c9d979bd0301175340546a832c6dd4e94164eff9c4bcabcae97fb0dcf215b	LIB
librist4_0.2.10+dfsg-2_amd64.deb	c8050889949e75064db8c9c25c7e41d4d2a79be7a60e6537f9d9ab54774495dd	LIB
librsvg2-2_2.58.0+dfsg-1build1_amd64.deb	47178c8632e70a0fd2c620e87c5d4d873b07f76c16e2e3c642693f8176a0af88	LIB
librubberband2_3.3.0+dfsg-2build1_amd64.deb	96ebaf773ad933a5269859cb3b09488b7b0b58387ebfbd7b7c469dd2b32d5d8b	LIB
libsamplerate0_0.2.2-4build1_amd64.deb	8a389d1cae81860c9fb7cd9641c3f9d38815e80c0379b3181b4317489126f738	LIB
libsdl2-2.0-0_2.30.0+dfsg-1ubuntu3.1_amd64.deb	a2e733f1339272f22cf7777d454b5d4f71e27335d89d4c99cf41e1ce1bb5c462	LIB
libselinux1_3.5-2ubuntu2.1_amd64.deb	6abaa6c26f46ef17764c4a753e0e84de1cdadde5634fd2987621fdc617988d19	LIB
libserd-0-0_0.32.2-1_amd64.deb	5ede35e66656c136b6a803aebeac92b2c061ec4d147820a730dd9c2a1315f57a	LIB
libsharpyuv0_1.3.2-0.4build3_amd64.deb	4e1763494e5b9d34192313aa88e38201fc67b3f13ea96ca268d26401a3002791	LIB
libshine3_3.1.1-2build1_amd64.deb	88a6e24ebb4be1c085acadb7778076de6980fa7e63b82570e29cc0f8a17a516b	LIB
libsixel1_1.10.3-3build1_amd64.deb	4340648a13a2ff8debe759ca7c94beb38fe739ea83402a193b82958824f053de	LIB
libslang2_2.3.3-3build2_amd64.deb	352deab8d4dcb023ecffc766088d9c5420c36a3bef9ef5b1c9b173d1c3c88bd7	LIB
libsnappy1v5_1.1.10-1build1_amd64.deb	c44dfa3b8be7c2873efb770890cb282a36a7e59b3b1900cb50d99f92e6be2da5	LIB
libsndfile1_1.2.2-1ubuntu5.24.04.1_amd64.deb	fe2461b27e747fa88ede157ea39c6379e2101c280a730cda9c825f566a3808e5	LIB
libsndio7.0_1.9.0-0.3build3_amd64.deb	b3944745ee48ea2c7a851fb78a617d34bb731a2f956a2c213cc9dffcaba35382	LIB
libsodium23_1.0.18-1ubuntu0.24.04.1_amd64.deb	74d6f8fbd65f7f71b6f4f4223bef5fb57c1c14cd4e9ce788ead2b88e8c3ee236	LIB
libsord-0-0_0.16.16-2build1_amd64.deb	a330fa014ee2eb48053395fb91fa39d95ca910e582d515254f50ef944240303c	LIB
libsoxr0_0.1.3-4build3_amd64.deb	2ddb6d21ad8ae0c850d380b551f109fc6b50c9a9b62c64ce37607c115bd3382a	LIB
libspeex1_1.2.1-2ubuntu2.24.04.1_amd64.deb	12d57caab1e3fa847cd385a21c1044f7d38209e97b0f87372fe41e346c2c17f7	LIB
libsphinxbase3t64_0.8+5prealpha+1-17build2_amd64.deb	80c184d04290cd1a55d6abac7c4b9f15bbf736e43a410857f992eba16b005a4c	LIB
libsratom-0-0_0.6.16-1build1_amd64.deb	a103c98906a7587718dabd52dfcd0af2dff6ff567781100bc8cf07de05bfc30d	LIB
libsrt1.5-gnutls_1.5.3-1build2_amd64.deb	77bf75aaa74be9552d5a3b05349d15dc6e00f3a854a12ad0dfffea0df3bd6523	LIB
libssh-gcrypt-4_0.10.6-2ubuntu0.4_amd64.deb	fcd374b0c64c3b2bb08dd33eec8b27f5e57f83c8e74d0bee9fe42c88762d6f75	LIB
libssl3t64_3.0.13-0ubuntu3.15_amd64.deb	3d0955bc049bbcca0f4c3e78a3a8b994593d96db7d84f4320217224433844534	LIB
libsvtav1enc1d1_1.7.0+dfsg-2build1_amd64.deb	bac5f768bff928e85cecb39da8e37276a579639ef884692b089ed2de6c1cd2ad	LIB
libswresample4_7%3a6.1.1-3ubuntu5_amd64.deb	d55afe23e07b02b2def1647181a727dfad996001f72b5172da8a99691c3fa208	LIB
libswscale7_7%3a6.1.1-3ubuntu5_amd64.deb	2c17ae58b35112aace5179d7deea51faa20e82e04847b257f339376b11744e22	LIB
libsystemd0_255.4-1ubuntu8.17_amd64.deb	4776d2ac7e21efe2ae31f3f7955a7ccd97277225eecf36910a26faf4544979ae	LIB
libtasn1-6_4.19.0-3ubuntu0.24.04.2_amd64.deb	f82c9ad142f952ea523bda5bfca7bb0802af3a50b8c0f0dd9ec18cdfa104bde7	LIB
libthai0_0.1.29-2build1_amd64.deb	5ada7045c5f84a4b774e6449151800aa6470e8d01dc9d6124ef4a44ae6af5508	LIB
libtheora0_1.1.1+dfsg.1-16.1build3_amd64.deb	8514bf3bb1496516b3bd0b2a5fc3ac9ea061e36360dcdfde52d51fa01c437b05	LIB
libtinfo6_6.4+20240113-1ubuntu2.1_amd64.deb	768e43c268e1e49b8dae7a50ed4778d6db88688abf72fdf779641e124c8a9a0b	LIB
libtwolame0_0.4.0-2build3_amd64.deb	0dc3f4ae19623d92eb2c7a131737355cc385047fb8891a6ae380e4884e3442c0	LIB
libuchardet0_0.0.8-1build1_amd64.deb	e2c390d8c1843059922f7ff3a74106c5af6fbf03c94532c07de16bf5af256fb4	LIB
libudev1_255.4-1ubuntu8.17_amd64.deb	6efea3770f738db2fd43ebdaed3d91ef0cde8aa8387b4803080af473326ebdb0	LIB
libudfread0_1.1.2-1build1_amd64.deb	9340c4bdb5954ed98a81eb7979c25187d0c4d95b633cac29eefe1f5b31c05162	LIB
libunibreak5_5.1-2build1_amd64.deb	62576c95652b6c0a02e3d0fe9e2da6d0666c375d71c43da9d884c77417956ae4	LIB
libunistring5_1.1-2build1.1_amd64.deb	203d7657b5f54633fba1a9c9b784d556ef83c9f6787b3185ba55a88e07b865a3	LIB
libusb-1.0-0_2%3a1.0.27-1_amd64.deb	d59e679a82ee2b724c556397e831e879936adbe8a007705872b65a5e1ec887b5	LIB
libva-drm2_2.20.0-2ubuntu0.2_amd64.deb	7919c889e0bc8221dcfb595bc0707959354a5404cc1d9b818f85fc43914ca0d4	LIB
libva-wayland2_2.20.0-2ubuntu0.2_amd64.deb	60752a6314017033066ccc12fea2f0911d5b224dd9e3b08470fae0b8f4f3abf6	LIB
libva-x11-2_2.20.0-2ubuntu0.2_amd64.deb	9b13aaf035c6b607885ffaa6688866cf3cb720637909c7c0c9d3bedcd1d723cf	LIB
libva2_2.20.0-2ubuntu0.2_amd64.deb	a8ff8d3d16dba715cd24d67c0f8787016194fcee0dee20344a92d057577c0b40	LIB
libvdpau1_1.5-2build1_amd64.deb	a071370329f6dfb954e0cd2ed28cadc17fbf1bdf30f6d1db1e178ad350844fb9	LIB
libvidstab1.1_1.1.0-2build1_amd64.deb	345caabe592ee7733c2142bdfb3fe18a97da4ee7372f57d574b698d58e3fd431	LIB
libvorbis0a_1.3.7-1build3_amd64.deb	d704ac0a5dd7a9797a2f2dc86fc8333c9973023b98b3510dcd2f1cc00da3624d	LIB
libvorbisenc2_1.3.7-1build3_amd64.deb	866c561fe81d4430a48c820f4e62f3cd138f4c8c7664f9203f02c1ced2eb55d4	LIB
libvorbisfile3_1.3.7-1build3_amd64.deb	fa12c52814bbfddff78186e7300b535bed30db02e4532fafae969aa9e5a79316	LIB
libvpl2_2023.3.0-1build1_amd64.deb	4f5d05ce3abb95a45d7b0d1e78e137f2849548dd461feb12d9066d550da0ca70	LIB
libvpx9_1.14.0-1ubuntu2.3_amd64.deb	a8555944c01f7308f3436b6f109328ebe680969e72066eab0a1f4c82a95e1740	LIB
libvulkan1_1.3.275.0-1build1_amd64.deb	ccf4fe8f4461442f27ea2494c7ae650b60bd396fec2688b0c44a27d66a222f74	LIB
libwayland-client0_1.22.0-2.1build1_amd64.deb	6af0aed41d75149bea22fa468f01eb058ffe3e35ef07ff2f13fb88a90387881d	LIB
libwayland-cursor0_1.22.0-2.1build1_amd64.deb	29c20da27e8e37c984cf0518635c11cd0441f4cb6a2919a9f6eee1809171d327	LIB
libwayland-egl1_1.22.0-2.1build1_amd64.deb	76cd17d2776428e0a0551b8f8fd954eeb3d39ca8ed2f88a38a5e4ce3ce3b2f16	LIB
libwebp7_1.3.2-0.4build3_amd64.deb	c86f6439f0bbc531f7199794654231f3b29327d6585958511b958795d51c2484	LIB
libwebpmux3_1.3.2-0.4build3_amd64.deb	0dd0f2de0fbbda7239209332ddfe512f4383d0489dd3f9fb25398f58c1621724	LIB
libx11-6_2%3a1.8.7-1build1_amd64.deb	397f84347476a3c5786b39f3ff6f0f82866eb3d8be6d2ad3efeadf019efe5b80	LIB
libx11-xcb1_2%3a1.8.7-1build1_amd64.deb	7d0d357e47cd6e1042be34da1d37cea313420b000035e71e855087c8268ab127	LIB
libx264-164_2%3a0.164.3108+git31e19f9-1_amd64.deb	7a410be6797c32357a2b7f0cc68ee14ae0e3b652bb17d16cdc941e4d830c7842	LIB
libx265-199_3.5-2build1_amd64.deb	795a6ca9287cd521882178d6503703ee8f0cde955f0ffc03e5ceee9f6d084de7	LIB
libxau6_1%3a1.0.9-1build6_amd64.deb	e40d29f1d1a62393bacaedebe0da3d9006084152a9f7e5e029293f08ce1c5c80	LIB
libxcb-dri3-0_1.15-1ubuntu2_amd64.deb	3d3d0e95e56ae16010a84883196b8153cba62d6831f9aaee2a68c46f20c7c004	LIB
libxcb-render0_1.15-1ubuntu2_amd64.deb	7d83a0668b1a693b5ea2e496206e8371fb6beda4c54a2b4b3902005915c272c5	LIB
libxcb-shape0_1.15-1ubuntu2_amd64.deb	ad57fed687ec9c2a4808b8e30ab1753d57daeddc9ad7fb883bfa0701f6da57c8	LIB
libxcb-shm0_1.15-1ubuntu2_amd64.deb	229d1280d459f1ba44c22939d3f9b61d9d20932d9a646b3fe4ce50be4cdf2325	LIB
libxcb-xfixes0_1.15-1ubuntu2_amd64.deb	0b2ab64af92a71e3d1a35e3c819880ee28d04bfac81360df68ae8d8e9663ebd2	LIB
libxcb1_1.15-1ubuntu2_amd64.deb	e1c6611d11ad7398326f1bf028afc34c3b14c51d917a3426b966ed4b9687fa58	LIB
libxcursor1_1%3a1.2.1-1build1_amd64.deb	1a4beee621454e42fe0d2f4e1633611126e43d891f0aec28e4a06744f6e9f04d	LIB
libxdmcp6_1%3a1.1.3-0ubuntu6_amd64.deb	bcd336fce11ce2a45f34d0f95e6980af22529f22147e8f98c156e5cee8ee42bb	LIB
libxext6_2%3a1.3.4-1build2_amd64.deb	45783969a9ece9d7b7b733b8c60981584c53c6bc5ee3b42d295d2f80d1285679	LIB
libxfixes3_1%3a6.0.0-2build1_amd64.deb	0ee1015cccd063249e01c0cd0bf45f513c8ac9a1e5e485070c12e69192455e4a	LIB
libxi6_2%3a1.8.1-1build1_amd64.deb	0ea7acb5e8a8ce4d6653b30e03a319bfe136db7bfb5ee7cad34c2aa1272ea8d9	LIB
libxkbcommon0_1.6.0-1build1_amd64.deb	2b9caeb423efb540296a1cb20b872cc630c23908407ecb5c1c787a617622d664	LIB
libxml2_2.9.14+dfsg-1.3ubuntu3.8_amd64.deb	bfd07c01d6e5ab3e327f3ca5819409b1914bbfb3f1a016d53e4dabd5f96143bb	LIB
libxpresent1_1.0.0-2build2_amd64.deb	75c31346edce1e1182e37353871257d0ea967cd3c12b6cdf9bd08b304b57ada0	LIB
libxrandr2_2%3a1.5.2-2build1_amd64.deb	f2955a5e594f5724b58ad241d9231ea191cb36574a0d5e5ca6b661cd41d6256d	LIB
libxrender1_1%3a0.9.10-1.1build1_amd64.deb	d70bd831aebe8d4834b5dd2ed98df26dd6bd27f1042c47543bd7f66df1ae22ea	LIB
libxss1_1%3a1.2.3-1build3_amd64.deb	0ac60a2cc034ccc4bf4e2f846f38110469d7ae43b47e808d67aafc538bc3695e	LIB
libxv1_2%3a1.0.11-1.1build1_amd64.deb	ddf5c60268d58fe0c57e80f241d6fa5e73ae2096a137c4d2de42f97cb379ece6	LIB
libxvidcore4_2%3a1.3.7-1build1_amd64.deb	4f70e6d3db3d3069797d84bccb082dbf6b14d6e837cbef6197519d8be8779cdd	LIB
libzimg2_3.0.5+ds1-1build1_amd64.deb	6c3d3e87277ff906011da1ceac5e44057df3710a53df3eab59675b02eea1ff12	LIB
libzix-0-0_0.4.2-2build1_amd64.deb	3bd3271411e9f25d03e16ca430da9551200b28e854ba9b606f819c23a4dc4992	LIB
libzmq5_4.3.5-1build2_amd64.deb	fff197d2a164e3ec83fef23afd55bad93971b3327d7353db9a305a199d55d402	LIB
libzstd1_1.5.5+dfsg2-2build1.1_amd64.deb	dfcf25061e07aad7efd3f4f880ba5ad4d4d09ebe7fc8cc77ab6b8a161d6d4727	LIB
libzvbi0t64_0.2.42-2_amd64.deb	0a596a628331e837b5756dec8bedf69ea699a9ef5d8a050c0c9b471f833039e1	LIB
mpv_0.37.0-1ubuntu4_amd64.deb	26b9273c3cc4c69b55ad908d168c624b95fd6785ea96625143e344b53e786e94	BIN
ocl-icd-libopencl1_2.3.2-1build1_amd64.deb	6d98029b06335bc23166834719ac5e697bcb4a136b40eebc263dfe481e05839b	LIB
"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

echo "==> downloading pinned mpv debs"
while read -r fname sha kind; do
    [ -n "$fname" ] || continue
    # launchpad files carry no epoch prefix (libavcodec60_7%3a6.1.1… →
    # libavcodec60_6.1.1…)
    url="https://launchpad.net/ubuntu/+archive/primary/+files/$(printf '%s' "$fname" | sed -E 's/_[0-9]+%3a/_/')"
    ok=""
    for attempt in 1 2 3; do
        curl -fsSL --retry 2 -o "$tmp_dir/$fname" "$url" && { ok=1; break; }
        sleep 2
    done
    [ -n "$ok" ] || { echo "error: failed to download $url" >&2; exit 1; }
    echo "$sha  $tmp_dir/$fname" | sha256sum -c - >/dev/null \
        || { echo "error: sha256 mismatch for $fname" >&2; exit 1; }
done <<<"$MANIFEST"

echo "==> assembling bundle"
stage="$tmp_dir/stage"
bundle="$tmp_dir/bundle"
mkdir -p "$stage" "$bundle"
for deb in "$tmp_dir"/*.deb; do
    dpkg-deb -x "$deb" "$stage"
done
find "$stage/usr/lib/x86_64-linux-gnu" -maxdepth 1 \( -type f -o -type l \) -name "*.so*" \
    -exec cp -a {} "$bundle/" \;
for sub in blas lapack pulseaudio; do
    cp -a "$stage/usr/lib/x86_64-linux-gnu/$sub/." "$bundle/" 2>/dev/null || true
done
cp "$stage/usr/bin/mpv" "$bundle/mpv"
# DT_RUNPATH covers only the object's own direct dependencies, so every
# shipped file carries $ORIGIN and the flat directory resolves regardless
# of load order.
find "$bundle" -maxdepth 1 -type f -exec patchelf --set-rpath '$ORIGIN' {} +

echo "==> verifying the bundle loader closure"
missing=""
for elf in "$bundle/mpv" "$bundle"/*.so.*; do
    [ -f "$elf" ] || continue
    unresolved=$(env -i LD_LIBRARY_PATH="$bundle" ldd "$elf" 2>/dev/null | grep "not found" || true)
    if [ -n "$unresolved" ]; then
        echo "error: unresolved dependencies in $(basename "$elf"):" >&2
        printf '%s\n' "$unresolved" >&2
        missing=1
    fi
done
[ -z "$missing" ] || { echo "error: the bundle loader closure is incomplete" >&2; exit 1; }
env -i "$bundle/mpv" --version >/dev/null

echo "==> installing $target"
rm -rf "$target"
mkdir -p "$target"
cp -a "$bundle/." "$target/"
{
    echo "# Pinned Ubuntu noble debs this mpv bundle was assembled from"
    echo "# (scripts/fetch-linux-mpv.sh; format: filename sha256 kind)"
    printf '%s\n' "$MANIFEST"
} > "$target/MANIFEST.txt"

"$0" --check
echo "==> ok: $target ($(du -sh "$target" | cut -f1))"
