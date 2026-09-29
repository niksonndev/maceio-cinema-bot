build-TelegramWebhookFunction:
	cargo lambda build --release --bin telegram-webhook
	mkdir -p "$(ARTIFACTS_DIR)"
	cp target/lambda/telegram-webhook/bootstrap "$(ARTIFACTS_DIR)/bootstrap"
	chmod 755 "$(ARTIFACTS_DIR)/bootstrap"

build-BotFunction:
	cargo lambda build --release --bin telegram-webhook
	mkdir -p "$(ARTIFACTS_DIR)"
	cp target/lambda/telegram-webhook/bootstrap "$(ARTIFACTS_DIR)/bootstrap"
	chmod 755 "$(ARTIFACTS_DIR)/bootstrap"

build-TelegramFetchFunction:
	cargo lambda build --release --bin telegram-fetch
	mkdir -p "$(ARTIFACTS_DIR)"
	cp target/lambda/telegram-fetch/bootstrap "$(ARTIFACTS_DIR)/bootstrap"
	chmod 755 "$(ARTIFACTS_DIR)/bootstrap"

build-FetchFunction:
	cargo lambda build --release --bin telegram-fetch
	mkdir -p "$(ARTIFACTS_DIR)"
	cp target/lambda/telegram-fetch/bootstrap "$(ARTIFACTS_DIR)/bootstrap"
	chmod 755 "$(ARTIFACTS_DIR)/bootstrap"