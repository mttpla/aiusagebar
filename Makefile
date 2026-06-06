CERT_NAME ?= AiUsageBar Dev
BINARY     = target/debug/aiusagebar

.PHONY: dev

dev:
	cargo build && codesign --force --sign "$(CERT_NAME)" $(BINARY) && $(BINARY)
