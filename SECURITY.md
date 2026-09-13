# Security Policy

## Reporting

Do not open a public issue for a suspected vulnerability. Use GitHub private vulnerability reporting for this repository. Include affected versions, impact, reproduction steps, and any suggested mitigation. Do not access third-party data, move funds, or test against deployments you do not control.

Maintainers will acknowledge a report within three business days, assess severity, coordinate a fix and retest, and publish an advisory when disclosure is appropriate. Reporters may request attribution.

## Supported versions

Only the latest tagged release is supported until a release policy states otherwise. Test fixture contracts are never production wallets or custody systems.

## Security boundaries

Contracts must never store report bodies, raw domains or URLs, credentials, KYC or customer data. They must not custody funds, execute fiat transfers, claim certification, or determine legal compliance. Privileged and publishing operations require explicit authorization. A public-network deployment requires documented governance approval and completed security review.
