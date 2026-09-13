# Deployment intelligence

StackPilot deployment intelligence recommends and generates production-minded deployment foundations. It does not become a hosted deployment platform, retain cloud credentials, operate workloads, or replace cloud-native operational tooling.

## First supported target: AWS ECS/Fargate

For a verified StackPilot backend golden path with Docker enabled and explicit AWS project metadata, `stackpilot inspect` can recommend **AWS ECS/Fargate** as an informational finding.

The recommendation is deterministic and does not change the `readiness-v1` score.

To preview the deployment foundation:

```bash
stackpilot fix . --deployment aws-ecs-fargate
```

To apply it:

```bash
stackpilot fix . --deployment aws-ecs-fargate --apply
```

The explicit deployment target implies AWS Terraform intent. Supplying a conflicting `--cloud Azure` or `--cloud GCP` is rejected.

## Generated foundation

The first ECS/Fargate adapter creates three additive Terraform files under `infra/terraform/`:

- `ecs-fargate.tf` — ECR, ECS cluster, task/service, IAM execution/task roles, CloudWatch logs, ALB, target group, listener, and security groups;
- `ecs-fargate-variables.tf` — VPC/subnet IDs, image URI, desired count, task sizing, public-IP behavior, listener CIDRs, log retention, and health-check grace period;
- `ecs-fargate-outputs.tf` — ECR URL, cluster/service names, ALB DNS name, and IAM role ARNs.

StackPilot intentionally accepts `vpc_id` and `subnet_ids` as inputs rather than owning a complete network topology. This lets teams use an existing VPC or a separately governed network foundation.

The application contract remains StackPilot's standard port `3000` and `/health` endpoint. The ECS target group uses `/health`, and the generated service enables the ECS deployment circuit breaker with rollback.

## Safety rules

Deployment remediation follows the existing `stackpilot fix` contract:

- preview is the default;
- `--apply` is required for writes;
- only verified StackPilot golden-path layouts are eligible;
- existing destination files are never overwritten;
- a partially present StackPilot deployment foundation is deferred for manual review rather than completed around unknown changes;
- existing Terraform must show AWS provider intent before ECS/Fargate files are added;
- source application files are not modified by deployment generation;
- a second preview after successful apply is idempotent.

## What StackPilot does not do

StackPilot does not continuously deploy, store AWS credentials, monitor running services, reconcile infrastructure forever, or choose operational policy on behalf of a team. Those concerns remain with CI/CD, Terraform/AWS, observability systems, and higher-level orchestration such as gODtECH FORGE.

Future deployment adapters may add other opinionated foundations such as App Runner, Azure Container Apps, and Google Cloud Run, but each adapter remains explicit, deterministic, and independently validated.
