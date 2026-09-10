pipeline {
    agent any

    environment {
        GITHUB_TOKEN = credentials('GITHUB_TOKEN')
        DOCKER_VOLS = '-v jenkins_jenkins_home:/var/jenkins_home -v cargo-registry-cache:/usr/local/cargo/registry'
        RUST_IMAGE = 'rust:latest'
        // No DB reachable in this container, so sqlx::query! macros must check
        // against the committed .sqlx/ offline cache instead of a live database.
        SQLX_OFFLINE = 'true'
    }

    stages {
        stage('Checkout') {
            steps {
                checkout scm
                sh '''
                curl -L \
                -X POST \
                -H "Accept: application/vnd.github+json" \
                -H "Authorization: Bearer $GITHUB_TOKEN" \
                -H "X-GitHub-Api-Version: 2022-11-28" \
                https://api.github.com/repos/tanndlin/rs-smtp/statuses/$GIT_COMMIT \
                -d '{"state":"pending","description":"Build in progress","context":"Jenkins"}'
                '''
            }
        }

        // stage('Lint') {
        //     steps {
        //         catchError(buildResult: 'FAILURE', stageResult: 'FAILURE') {
        //             sh '''
        //             docker run --rm $DOCKER_VOLS -w $WORKSPACE $RUST_IMAGE \
        //                 sh -c "rustup component add clippy && cargo clippy --workspace --all-targets -- -D clippy::pedantic"
        //             '''
        //         }
        //     }
        // }

        stage('Format Check') {
            steps {
                catchError(buildResult: 'FAILURE', stageResult: 'FAILURE') {
                    sh '''
                    docker run --rm $DOCKER_VOLS -w $WORKSPACE $RUST_IMAGE \
                        sh -c "rustup component add rustfmt && cargo fmt --all -- --check"
                    '''
                }
            }
        }

        stage('Build') {
            steps {
                sh '''
                docker run --rm $DOCKER_VOLS -e SQLX_OFFLINE -w $WORKSPACE $RUST_IMAGE \
                    sh -c "cargo build --workspace --release"
                '''
            }
        }

        stage('Test') {
            steps {
                // The integration tests in imap/tests/ open a real Postgres
                // connection at runtime (SQLX_OFFLINE only covers compile-time
                // query checks), so stand up a throwaway DB on a private network
                // and point the tests at it via TEST_DATABASE_URL. The trap
                // tears everything down even when cargo test fails.
                sh '''
                set -e
                CI_ID=$(echo "${JOB_NAME}_${BUILD_NUMBER}" | tr -c 'a-zA-Z0-9_.-' '-')
                NET="ci-net-$CI_ID"
                DB="ci-db-$CI_ID"

                cleanup() {
                    docker rm -f "$DB" >/dev/null 2>&1 || true
                    docker network rm "$NET" >/dev/null 2>&1 || true
                }
                trap cleanup EXIT

                docker network create "$NET" || true
                docker run -d --rm --name "$DB" --network "$NET" \
                    -e POSTGRES_USER=user -e POSTGRES_PASSWORD=password -e POSTGRES_DB=postgres \
                    postgres:15-alpine

                for i in $(seq 1 30); do
                    docker exec "$DB" pg_isready -U user -d postgres >/dev/null 2>&1 && break
                    sleep 1
                done

                docker run --rm $DOCKER_VOLS --network "$NET" -e SQLX_OFFLINE \
                    -e TEST_DATABASE_URL=postgres://user:password@$DB:5432/postgres \
                    -e DATABASE_URL=postgres://user:password@$DB:5432/postgres \
                    -w $WORKSPACE $RUST_IMAGE \
                    sh -c "cargo test --workspace"
                '''
            }
        }
    }

    post {
        success {
            sh '''
            curl -L \
            -X POST \
            -H "Accept: application/vnd.github+json" \
            -H "Authorization: Bearer $GITHUB_TOKEN" \
            -H "X-GitHub-Api-Version: 2022-11-28" \
            https://api.github.com/repos/tanndlin/rs-smtp/statuses/$GIT_COMMIT \
            -d '{"state":"success","description":"Build succeeded","context":"Jenkins"}'
            '''
        }
        failure {
            sh '''
            curl -L \
            -X POST \
            -H "Accept: application/vnd.github+json" \
            -H "Authorization: Bearer $GITHUB_TOKEN" \
            -H "X-GitHub-Api-Version: 2022-11-28" \
            https://api.github.com/repos/tanndlin/rs-smtp/statuses/$GIT_COMMIT \
            -d '{"state":"failure","description":"Build failed","context":"Jenkins"}'
            '''
        }
    }
}
