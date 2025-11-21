ARG SLURM_VERSION=@SLURM_VERSION@
COPY install_slurm.sh /
RUN  /install_slurm.sh
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH=/root/.cargo/bin:$PATH
COPY entrypoint.sh /
CMD ["bash"]
