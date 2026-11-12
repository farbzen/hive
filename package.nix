{
  lib,
  rustPlatform,
  fetchFromGitHub,
  makeWrapper,
  git,
  colmena,
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "hive";
  version = "1.0.0";

  src = fetchFromGitHub {
    owner = "farbzen";
    repo = "hive";
    tag = "v${finalAttrs.version}";
    hash = "";
  };

  cargoHash = "";

  nativeBuildInputs = [makeWrapper];

  postInstall = ''
    for program in $out/bin/hive $out/bin/hive-tui; do
      wrapProgram "$program" --prefix PATH : ${lib.makeBinPath [git colmena]}
    done
  '';

  meta = {
    description = "Hive Infrastructure Versioning Engine - Deploying colmena hives and creating commits";
    homepage = "https://github.com/farbzen/hive";
    changelog = "https://github.com/farbzen/hive/blob/v${finalAttrs.version}/CHANGELOG.md";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "hive";
    maintainers = [
      {
        email = "dev@farbzen.de";
        github = "farbzen";
        githubId = 60570640;
        name = "Fabian Meier";
      }
    ];
  };
})
