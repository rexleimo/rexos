FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    bash \
    ca-certificates \
    chromium \
    curl \
    fonts-liberation \
    fonts-noto-color-emoji \
    novnc \
    python3 \
    socat \
    websockify \
    x11vnc \
    xvfb \
  && rm -rf /var/lib/apt/lists/*

COPY docker/sandbox-browser/entrypoint.sh /usr/local/bin/rexos-sandbox-browser
RUN chmod +x /usr/local/bin/rexos-sandbox-browser

RUN useradd --create-home --shell /bin/bash sandbox
USER sandbox
WORKDIR /home/sandbox

EXPOSE 9222 5900 6080

CMD ["rexos-sandbox-browser"]
