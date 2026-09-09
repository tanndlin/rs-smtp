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
                sh '''
                docker run --rm $DOCKER_VOLS -e SQLX_OFFLINE -w $WORKSPACE $RUST_IMAGE \
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
