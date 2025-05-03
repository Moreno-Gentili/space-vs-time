FROM clockworklabs/spacetime:v1.1.0
ENV DOTNET_ARCH=amd64
ENV DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1
ENV PATH=/home/spacetime/.local/bin:/usr/share/dotnet:/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
COPY --chown=spacetime:spacetime --chmod=777 . /app/server
WORKDIR /app/server
RUN bash -c "source /usr/local/cargo/env && cargo build --target wasm32-unknown-unknown --release"
WORKDIR /app

EXPOSE 3000
ENTRYPOINT ["/app/server/start.sh"]