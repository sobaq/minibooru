{
  inputs.nixpkgs.url = "nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
  let pkgs = nixpkgs.legacyPackages.x86_64-linux;
  in {
      devShells.x86_64-linux.default = pkgs.mkShell {
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

          buildInputs = with pkgs; [
            # cargo rustc
            postgresql sqlx-cli

            ffmpeg_7-full pkg-config clang

            # for statically linking ffmpeg
            bzip2 xz lame libtheora libogg xvidcore soxr libvdpau
            xorg.libX11 nasm
          ];
        };
    };
}
