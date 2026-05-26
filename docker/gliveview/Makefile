IMAGE := gliveview

.PHONY: start stop build

start: build
	docker run --rm -it \
		--add-host host.docker.internal:host-gateway \
		-v gliveview-blocklog:/opt/cardano/blocklog \
		$(IMAGE)

stop:
	docker stop $$(docker ps -q --filter ancestor=$(IMAGE)) 2>/dev/null || true

build:
	docker build -t $(IMAGE) .
