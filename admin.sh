#!/bin/bash
# ===============================================
# 🚀 Gestionnaire Code Terroir (API + GUI)
# ===============================================

APP_DIR="$HOME/dev/code_terroir_warp"
#APP_DIR="~/dev/code_terroir_warp"
LOG_DIR="$APP_DIR/logs"
API_LOG="$LOG_DIR/code-terroir-api.log"
GUI_LOG="$LOG_DIR/code-terroir-gui.log"
API_PORT=3030
GUI_PORT=8081

# Créer le dossier logs si besoin
mkdir -p "$LOG_DIR"

cd "$APP_DIR" || { echo "❌ Impossible de se placer dans $APP_DIR"; exit 1; }

# -----------------------------------------------
# 🔧 Fonctions utilitaires
# -----------------------------------------------
get_pid() {
  pgrep -f "$1" | head -n 1
}

api_pid() {
  get_pid "target/debug/code-terroir"
}

api_pid_release() {
  get_pid "target/release/code-terroir"
}

gui_pid() {
  get_pid "serve_gui.py"
}

# -----------------------------------------------
# 🚀 Commandes principales
# -----------------------------------------------
start() {
  echo "🚀 Démarrage de l'API Rust (port $API_PORT)..."
  nohup cargo run > "$API_LOG" 2>&1 &
  sleep 2
  local pid=$(api_pid)
  if [ -n "$pid" ]; then
    echo "✅ API lancée (PID $pid)"
  else
    echo "❌ Échec du démarrage de l’API"
  fi

  echo "🌱 Démarrage du GUI sur http://localhost:$GUI_PORT ..."
  nohup python3 serve_gui.py > "$GUI_LOG" 2>&1 &
  sleep 2
  local gpid=$(gui_pid)
  if [ -n "$gpid" ]; then
    echo "✅ GUI lancé (PID $gpid)"
  else
    echo "❌ Échec du démarrage du GUI"
  fi
}

start_release() {
  echo "🚀 Compilation et lancement en mode RELEASE..."
  cargo build --release || { echo "❌ Erreur compilation release"; exit 1; }
  nohup target/release/code-terroir > "$API_LOG" 2>&1 &
  sleep 2
  local pid=$(api_pid_release)
  if [ -n "$pid" ]; then
    echo "✅ API (release) lancée (PID $pid)"
  else
    echo "❌ Erreur au lancement release"
  fi
}

stop() {
  echo "🛑 Arrêt des services..."
  pkill -f "target/debug/code-terroir" && echo "✅ API stoppée"
  pkill -f "target/release/code-terroir" && echo "✅ API release stoppée"
  pkill -f "serve_gui.py" && echo "✅ GUI stoppé"
}

status() {
  echo "─────────────────────────────"
  echo "🌐 STATUS CODE TERROIR"
  echo "─────────────────────────────"
  local gpid=$(gui_pid)
  local dpid=$(api_pid)
  local rpid=$(api_pid_release)

  if [ -n "$gpid" ]; then
    echo "✅ GUI actif (PID $gpid) – Port $GUI_PORT"
  else
    echo "❌ GUI arrêté"
  fi

  if [ -n "$dpid" ]; then
    echo "✅ API active (debug) (PID $dpid) – Port $API_PORT"
  elif [ -n "$rpid" ]; then
    echo "✅ API active (release) (PID $rpid) – Port $API_PORT"
  else
    echo "❌ API arrêtée"
  fi
}

restart() {
  stop
  sleep 1
  start
}

build_release() {
  echo "🧱 Compilation en mode RELEASE..."
  cargo build --release
}

clean() {
  echo "🧹 Nettoyage complet du dossier target/"
  cargo clean
}

logs() {
  echo "─────────────────────────────"
  echo "📜 Dernières lignes des logs"
  echo "─────────────────────────────"
  tail -n 20 -f "$API_LOG" "$GUI_LOG"
}

# -----------------------------------------------
# 🧭 Menu principal
# -----------------------------------------------
case "$1" in
  start) start ;;
  start-release|release) start_release ;;
  stop) stop ;;
  restart) restart ;;
  status) status ;;
  build) cargo build ;;
  build-release) build_release ;;
  clean) clean ;;
  logs) logs ;;
  *)
    echo "Usage: $0 {start|start-release|stop|restart|status|build|build-release|clean|logs}"
    ;;
esac
