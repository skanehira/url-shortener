FROM ghcr.io/actions/actions-runner:latest

# rootに切り替えてパッケージインストール
USER root
RUN apt-get update && apt-get install -y \
    make \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# runnerユーザーに戻す（重要）
USER runner
WORKDIR /home/runner
