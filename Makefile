USERNAME := $(shell whoami)
HOME_DIR := $(shell echo $$HOME)
BINARY := github-notifier
INSTALL_PATH := /usr/local/bin
INSTALL_ASSETS_PATH := $(HOME_DIR)/.config/github-notifier
GITHUB_TOKEN := $(shell echo $$GITHUB_TOKEN)

AUTOSTART_DIR = $(HOME)/.config/autostart
AUTOSTART_FILE = $(AUTOSTART_DIR)/github-notifier.desktop
AUTOSTART_TEMPLATE = ./github-notifier.desktop.in

.PHONY: build build-debug test run clean install uninstall

build:
	cargo build --release
	cp target/release/$(BINARY) ./
	chmod +x ./$(BINARY)

build-debug:
	cargo build
	cp target/debug/$(BINARY) ./
	chmod +x ./$(BINARY)


test:
	cargo test

run: build
	./$(BINARY)

clean:
	cargo clean
	rm -f $(BINARY)

install: build
	@mkdir -p $(AUTOSTART_DIR)
	sed -e 's|{{USER}}|$(USERNAME)|g' \
	    -e 's|{{GITHUB_TOKEN}}|$(GITHUB_TOKEN)|g' \
	    -e 's|{{WORK_DIR}}|$(INSTALL_ASSETS_PATH)|g' \
	    -e 's|{{EXEC}}|$(INSTALL_PATH)/$(BINARY)|g' \
	    $(AUTOSTART_TEMPLATE) > $(AUTOSTART_FILE)
	sudo cp ./$(BINARY) $(INSTALL_PATH)/
	sudo chmod +x $(INSTALL_PATH)/$(BINARY)
	sudo cp -r ./assets $(INSTALL_ASSETS_PATH)/
	@echo "Autostart entry installed for user: $(USERNAME)"


uninstall:
	rm -f $(AUTOSTART_FILE)
	rm -f $(INSTALL_PATH)/$(BINARY)
	@echo "Autostart entry removed."

logs:
	tail -f /tmp/github-notifier.log