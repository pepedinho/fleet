# Nom du projet
PROJECT_NAME=fleet
SERVICE_NAME=fleetd
CARGO_BIN_DIR=$(HOME)/.cargo/bin
CONFIG_DIR=$(HOME)/.config/$(PROJECT_NAME)
SERVICE_PATH=$(HOME)/.config/systemd/user/$(SERVICE_NAME).service

.PHONY: install uninstall update build

install: build install-service
	@echo "✅ $(PROJECT_NAME) installed successfully."

build:
	@echo "📦 Building project in release mode..."
	cargo install --path . --force

install-service:
	@echo "⚙ Creating and installing systemd service..."
	mkdir -p $(HOME)/.config/systemd/user
	printf "[Unit]\nDescription=Fleet Daemon\n\n[Service]\nExecStart=%s/fleetd\nRestart=always\n\n[Install]\nWantedBy=default.target\n" "$(CARGO_BIN_DIR)" > $(SERVICE_PATH)
	systemctl --user daemon-reload
	systemctl --user enable $(SERVICE_NAME)
	systemctl --user start $(SERVICE_NAME)

uninstall: uninstall-service
	@echo "🗑 Removing installed binaries..."
	cargo uninstall fleet || true
	@echo "🗑 Removing configuration..."
	rm -rf $(CONFIG_DIR)
	@echo "✅ Uninstalled successfully."

uninstall-service:
	@echo "⚙ Stopping and disabling systemd service..."
	systemctl --user stop $(SERVICE_NAME) || true
	systemctl --user disable $(SERVICE_NAME) || true
	rm -f $(SERVICE_PATH)
	systemctl --user daemon-reload

update:
	@echo "🔄 Updating project..."
	cargo install --path . --force
	systemctl --user restart $(SERVICE_NAME)
