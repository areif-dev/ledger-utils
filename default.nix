{
  lib,
  rustPlatform,
  fetchFromGitHub,
}:

rustPlatform.buildRustPackage rec {
  pname = "pta-template-engine";
  version = "0.1.0";

  src = fetchFromGitHub {
    owner = "areif-dev";
    repo = "pta-template-engine";
    rev = "4f28ec28143b256da92ec10cb055ec6a0d8b8161";
    hash = "sha256-0DWlehZhB8hE4JJHjD/e7p+tO6W8JhwSgrMlqs0k4Pg=";
  };

  cargoHash = "sha256-riLzjP2MtJeyPCYEDW6qI59FGsyR2feXRoeaayjG9go=";

  meta = {
    homepage = "https://github.com/areif-dev/pta-template-engine";
    description = "Automate creating complex plaintext accounting journal entries with Jinja2 style templates";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ areif-dev ];
    mainProgram = "ptatemp";
    platforms = lib.platforms.linux;
  };
}
