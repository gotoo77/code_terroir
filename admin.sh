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
API_START_TIMEOUT=45

# Créer le dossier logs si besoin
mkdir -p "$LOG_DIR"

cd "$APP_DIR" || { echo "❌ Impossible de se placer dans $APP_DIR"; exit 1; }

# -----------------------------------------------
# 🔧 Fonctions utilitaires
# -----------------------------------------------
get_pid() {
  pgrep -f -- "$1" | head -n 1
}

api_pid() {
  get_pid "target/debug/[c]ode-terroir"
}

api_pid_release() {
  get_pid "target/release/[c]ode-terroir"
}

gui_pid() {
  get_pid "[s]erve_gui.py"
}

api_healthy() {
  curl --fail --silent --max-time 2 "http://127.0.0.1:$API_PORT/health" >/dev/null 2>&1
}

gui_healthy() {
  curl --fail --silent --max-time 2 "http://127.0.0.1:$GUI_PORT/" >/dev/null 2>&1
}

wait_for_api() {
  local launch_pid="$1"
  local attempt
  for ((attempt = 1; attempt <= API_START_TIMEOUT; attempt++)); do
    if api_healthy; then
      return 0
    fi
    if ! kill -0 "$launch_pid" 2>/dev/null; then
      return 1
    fi
    sleep 1
  done
  return 1
}

wait_for_gui() {
  local launch_pid="$1"
  local attempt
  for ((attempt = 1; attempt <= 10; attempt++)); do
    if gui_healthy; then
      return 0
    fi
    if ! kill -0 "$launch_pid" 2>/dev/null; then
      return 1
    fi
    sleep 1
  done
  return 1
}

start_dependencies() {
  echo "🗄️  Démarrage de PostgreSQL..."
  if ! docker compose up -d postgres; then
    echo "❌ Impossible de démarrer PostgreSQL"
    return 1
  fi

  if command -v redis-cli >/dev/null 2>&1 && redis-cli ping 2>/dev/null | grep -q '^PONG$'; then
    echo "✅ Redis local déjà disponible – Port 6379"
  else
    echo "🗄️  Démarrage de Redis..."
    if ! docker compose up -d redis; then
      echo "❌ Impossible de démarrer Redis"
      return 1
    fi
  fi
}

show_api_failure() {
  echo "❌ Échec du démarrage de l’API"
  if [ -s "$API_LOG" ]; then
    echo "   Dernière erreur :"
    tail -n 8 "$API_LOG" | sed 's/^/   /'
  fi
}

# -----------------------------------------------
# 🚀 Commandes principales
# -----------------------------------------------
start() {
  start_dependencies || return 1

  if api_healthy; then
    echo "✅ API déjà disponible (PID $(api_pid)) – Port $API_PORT"
  else
    echo "🚀 Démarrage de l'API Rust (port $API_PORT)..."
    nohup cargo run > "$API_LOG" 2>&1 &
    local launch_pid=$!
    if wait_for_api "$launch_pid"; then
      echo "✅ API disponible (PID $(api_pid)) – Port $API_PORT"
    else
      show_api_failure
      return 1
    fi
  fi

  if gui_healthy && [ -n "$(gui_pid)" ]; then
    echo "✅ GUI déjà actif (PID $(gui_pid)) – Port $GUI_PORT"
  else
    echo "🌱 Démarrage du GUI sur http://localhost:$GUI_PORT ..."
    nohup python3 serve_gui.py > "$GUI_LOG" 2>&1 &
    local gui_launch_pid=$!
    if wait_for_gui "$gui_launch_pid"; then
      echo "✅ GUI lancé (PID $(gui_pid)) – Port $GUI_PORT"
    else
      echo "❌ Échec du démarrage du GUI"
      return 1
    fi
  fi
}

start_release() {
  start_dependencies || return 1
  echo "🚀 Compilation et lancement en mode RELEASE..."
  cargo build --release || { echo "❌ Erreur compilation release"; exit 1; }
  nohup target/release/code-terroir > "$API_LOG" 2>&1 &
  local launch_pid=$!
  if wait_for_api "$launch_pid"; then
    echo "✅ API release disponible (PID $(api_pid_release)) – Port $API_PORT"
  else
    show_api_failure
    return 1
  fi
}

stop() {
  echo "🛑 Arrêt des services..."
  if pkill -f "target/debug/code-terroir"; then echo "✅ API stoppée"; fi
  if pkill -f "target/release/code-terroir"; then echo "✅ API release stoppée"; fi
  if pkill -f "serve_gui.py"; then echo "✅ GUI stoppé"; fi
}

status() {
  echo "─────────────────────────────"
  echo "🌐 STATUS CODE TERROIR"
  echo "─────────────────────────────"
  local gpid=$(gui_pid)
  local dpid=$(api_pid)
  local rpid=$(api_pid_release)

  if gui_healthy && [ -n "$gpid" ]; then
    echo "✅ GUI actif (PID $gpid) – Port $GUI_PORT"
  else
    echo "❌ GUI arrêté"
  fi

  if api_healthy && [ -n "$dpid" ]; then
    echo "✅ API disponible (debug) (PID $dpid) – Port $API_PORT"
  elif api_healthy && [ -n "$rpid" ]; then
    echo "✅ API disponible (release) (PID $rpid) – Port $API_PORT"
  elif [ -n "$dpid$rpid" ]; then
    echo "⚠️  Processus API actif, mais /health ne répond pas"
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
