ARG GLAB_CLI_VERSION="1.78.2"
RUN curl -sOL https://gitlab.com/gitlab-org/cli/-/releases/v${GLAB_CLI_VERSION}/downloads/glab_${GLAB_CLI_VERSION}_linux_amd64.rpm
RUN zypper install --allow-unsigned-rpm -y bind-utils openssh ./glab_${GLAB_CLI_VERSION}_linux_amd64.rpm
