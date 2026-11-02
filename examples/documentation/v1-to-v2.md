+++
unit = "billing"
from = "v1"
to = "v2"
title = "Allow multiple payments per invoice"
reviewed = true
impact = "Older integrations must no longer assume a single payment intent per invoice."
upgrade = "Use the payments collection and review callers that read payment_intent."
+++

## What's new

An invoice can be associated with more than one payment.

## Request template

```http
GET /invoices/{id} HTTP/1.1
X-API-Version: 2026-04-01
```
