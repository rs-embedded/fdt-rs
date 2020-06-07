gitlab-runner exec docker docs &&\
gitlab-runner exec docker clippy &&\
gitlab-runner exec docker test-stable &&\
gitlab-runner exec docker test-nightly
