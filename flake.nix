{
  description = "typst-ansi-hl highlights your Typst code";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;

        # Common arguments can be set here to avoid repeating them later
        # Note: changes here will rebuild all dependency crates
        commonArgs = {
          inherit src;
          strictDeps = true;

          buildInputs = [
            # Add additional build inputs here
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        typst-ansi-hl = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          doCheck = true;
          meta.mainProgram = "typst-ansi-hl";
        });
      in
      {
        checks = {
          inherit typst-ansi-hl;

          typst-ansi-hl_fmt = craneLib.cargoFmt {
            inherit src;
          };

          typst-ansi-hl_clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
          });

          typst-ansi-hl_test = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        packages.default = typst-ansi-hl;

        apps.default = flake-utils.lib.mkApp {
          drv = typst-ansi-hl;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};
        };
      });
}
