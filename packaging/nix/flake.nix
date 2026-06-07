{
  description = "Amaru binary distribution";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
    in
    flake-utils.lib.eachSystem supportedSystems (system:
      let
        pkgs = import nixpkgs { inherit system; };
        release = {
          x86_64-linux = {
            archive = "amaru-10.10.20260607-linux-x86_64.tar.gz";
            hash = "sha256-0pUXqUA2xv4G0Hb9kxamUIEGKz1dw60J+4e4+7Zv0lQ=";
          };
          aarch64-linux = {
            archive = "amaru-10.10.20260607-linux-aarch64.tar.gz";
            hash = "sha256-2BtOIBEngip3MxrpDjJEjObaRR6z0wsV2BR5MbSUfCc=";
          };
          aarch64-darwin = {
            archive = "amaru-10.10.20260607-macos-aarch64.tar.gz";
            hash = "sha256-AStYdmzXihstbh6QNOCNMYGYXkoLiH3zf6Zcb5ufRmw=";
          };
        }.${system};

        amaru = pkgs.stdenvNoCC.mkDerivation {
          pname = "amaru";
          version = "10.10.20260607";
          src = pkgs.fetchurl {
            url = "https://github.com/KtorZ/amaru/releases/download/v10.10.20260607/${release.archive}";
            hash = release.hash;
          };

          dontConfigure = true;
          dontBuild = true;

          unpackPhase = ''
            runHook preUnpack
            mkdir extracted
            tar -xzf "$src" -C extracted --strip-components=1
            cd extracted
            chmod -R u+w .
            runHook postUnpack
          '';

          installPhase = ''
            runHook preInstall
            mkdir -p "$out"
            cp -R . "$out/"
            chmod +x "$out/bin/amaru"
            runHook postInstall
          '';

          meta = with pkgs.lib; {
            description = "A Cardano blockchain node implementation";
            homepage = "https://github.com/pragma-org/amaru";
            license = licenses.asl20;
            mainProgram = "amaru";
            platforms = [ system ];
          };
        };
        amaruApp = {
          type = "app";
          program = "${amaru}/bin/amaru";
        };
      in {
        packages = {
          amaru = amaru;
          default = amaru;
        };

        apps = {
          amaru = amaruApp;
          default = amaruApp;
        };
      })
    // {
      overlays.default = final: prev: {
        amaru = self.packages.${final.system}.amaru;
      };
    };
}
