{ config, ... }:

let
  home = "/Users/${config.system.primaryUser}";
in
{
  launchd.user.agents.n8n = {
    serviceConfig = {
      Label = "io.n8n.n8n";
      ProgramArguments = [ "/nix/var/nix/profiles/system/sw/bin/n8n" "start" ];
      RunAtLoad = true;
      KeepAlive = true;
      StandardOutPath = "/tmp/n8n.log";
      StandardErrorPath = "/tmp/n8n.err";
      # Adjust this to wherever you clone Eggocado D2V
      WorkingDirectory = "${home}/Documents/GitHub/Eggocado-D2V";
      EnvironmentVariables = {
        HOME = home;
        N8N_USER_FOLDER = "${home}/.n8n";
        N8N_PORT = "5679";
      };
    };
  };

  launchd.user.agents.kokoro = {
    serviceConfig = {
      Label = "io.github.remsky.kokoro-fastapi";
      ProgramArguments = [
        "/opt/homebrew/bin/uv" "run" "--no-sync"
        "uvicorn" "api.src.main:app"
        "--host" "0.0.0.0"
        "--port" "8881"
      ];
      RunAtLoad = true;
      KeepAlive = true;
      StandardOutPath = "/tmp/kokoro.log";
      StandardErrorPath = "/tmp/kokoro.err";
      # Adjust this to wherever you clone Kokoro-FastAPI
      WorkingDirectory = "${home}/Documents/GitHub/Kokoro-FastAPI";
      EnvironmentVariables = {
        HOME = home;
        USE_GPU = "true";
        DEVICE_TYPE = "mps";
        PYTORCH_ENABLE_MPS_FALLBACK = "1";
        PYTHONPATH = "${home}/Documents/GitHub/Kokoro-FastAPI:${home}/Documents/GitHub/Kokoro-FastAPI/api";
        MODEL_DIR = "src/models";
        VOICES_DIR = "src/voices/v1_0";
        WEB_PLAYER_PATH = "${home}/Documents/GitHub/Kokoro-FastAPI/web";
        PORT = "8881";
      };
    };
  };
}
