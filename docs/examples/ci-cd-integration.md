# Real-World Scenario: CI/CD Pipeline Integration

**Document Reference:** `PORTPROBE-DOC-EXM-04`  
**Document Version:** `2.0.0`  
**Classification:** Enterprise Scenario Manual  
**Status:** Official Release  
**Last Updated:** November 2026  

---

## 1. Business Context & Objective

**Business Problem**: In automated deployment pipelines (GitLab CI, GitHub Actions, Jenkins), application deployment stages frequently fail with cryptic timeouts because a cloud security group or database firewall port was not provisioned. Pipelines spend 15–20 minutes attempting container rollouts before failing.

**Objective**: Embed a lightweight, deterministic network pre-flight check into the CI/CD pipeline using `portprobe --health-check` to fail fast within 3 seconds if target dependencies are unreachable.

---

## 2. GitHub Actions Integration Example

### Pipeline Configuration (`.github/workflows/deploy.yml`):
```yaml
name: Production Deployment Pipeline

on:
  push:
    branches: [ "main" ]

jobs:
  network-preflight-gate:
    name: Network Pre-Flight Reachability Gate
    runs-on: ubuntu-latest
    steps:
      - name: Checkout Code
        uses: actions/checkout@v4

      - name: Install PortProbe Binary
        run: |
          curl -sSL https://github.com/portprobe/portprobe/releases/latest/download/portprobe-linux-amd64 -o /usr/local/bin/portprobe
          chmod +x /usr/local/bin/portprobe

      - name: Verify Database Reachability (Port 5432)
        run: |
          portprobe --health-check --bind "10.200.5.50" --port 5432

      - name: Verify Redis Cache Reachability (Port 6379)
        run: |
          portprobe --health-check --bind "10.200.5.60" --port 6379

  deploy-application:
    name: Deploy Application Workloads
    needs: network-preflight-gate
    runs-on: ubuntu-latest
    steps:
      - name: Execute Kubernetes Rollout
        run: |
          echo "Network reachability confirmed. Rolling out containers..."
          kubectl rollout restart deployment/api-gateway -n production
```

---

## 3. GitLab CI Integration Example

### Pipeline Configuration (`.gitlab-ci.yml`):
```yaml
stages:
  - preflight
  - deploy

network-validation:
  stage: preflight
  image: alpine:3.20
  before_script:
    - apk add --no-cache curl
    - curl -sSL https://github.com/portprobe/portprobe/releases/latest/download/portprobe-linux-amd64 -o /usr/local/bin/portprobe
    - chmod +x /usr/local/bin/portprobe
  script:
    - echo "Validating core banking database reachability..."
    - portprobe --health-check --bind "$DB_HOST" --port "$DB_PORT"
  rules:
    - if: '$CI_COMMIT_BRANCH == "main"'
```

### Result:
If the database port is closed, the pipeline fails immediately at the `preflight` stage with exit code `1`, alerting engineers before any workloads are disrupted.
