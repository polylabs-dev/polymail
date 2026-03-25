# Poly Mail Enterprise Bundle

**Version**: 1.0
**Last Updated**: February 2026
**Classification**: CONFIDENTIAL

---

## Overview

Poly Mail is the anchor product for the "Poly Labs for Business" enterprise offering. It provides the most familiar entry point for enterprises (email) while establishing Poly OAuth as the identity layer that creates long-term lock-in.

---

## Enterprise Tiers

### Business ($12.99/user/mo)
- Custom domain (1)
- 50GB per user
- Admin console (basic)
- Scatter: 3-of-5
- SPARK biometric auth
- Compliance: 1-year retention
- Support: Email, 48hr response

### Enterprise ($24.99/user/mo)
- Custom domains (unlimited)
- 200GB per user
- Admin console (full)
- Scatter: 5-of-7, region-locked
- Poly OAuth SSO
- Compliance: Custom retention, legal hold, eDiscovery (MPC)
- DLP: Classification-based
- Migration tools
- Support: Priority, 4hr response
- SLA: 99.99%

### Sovereign (Contract)
- All Enterprise features
- Scatter: 9-of-13, 5+ jurisdictions
- HSM-backed (Poly Vault)
- Dedicated relay infrastructure
- On-premise option (eStream operator license)
- Custom compliance certifications
- Dedicated account manager
- Support: 24/7, 1hr response
- SLA: 99.999%

---

## Bundle: Poly Labs for Business

Combine products for enterprise discount:

| Bundle | Includes | Price |
|--------|----------|-------|
| Communication | Mail + Messenger | $18.99/user/mo |
| Productivity | Mail + Messenger + Data | $29.99/user/mo |
| Security | Mail + Messenger + Pass + VPN | $34.99/user/mo |
| Complete | All Poly products | $44.99/user/mo |
| Sovereign | All + HSM + Dedicated | Contract |

---

## Migration Path

### From Google Workspace
1. Domain verification (MX, DKIM, SPF)
2. Bulk email import via Google Takeout -> Poly Mail importer
3. Contact import (vCard)
4. Calendar import (iCal) -- when Poly Calendar available
5. Drive migration -> Poly Data
6. Gradual cutover (dual-delivery period)

### From Microsoft 365
1. Domain verification
2. Bulk email import via PST/EML export -> Poly Mail importer
3. Contact import (vCard/CSV)
4. OneDrive migration -> Poly Data
5. Azure AD -> Poly OAuth migration (SCIM provisioning)

### From Proton for Business
1. Domain transfer
2. Email import via Proton export -> Poly Mail importer
3. Proton Drive -> Poly Data
4. Proton Pass -> Poly Pass

---

## Compliance Features

| Feature | How |
|---------|-----|
| Retention policies | FLIR FSM enforces per-classification retention |
| Legal hold | Prevents deletion/expiry for specified users/timeframes |
| eDiscovery | MPC-based: prove existence/content without revealing other data |
| Audit logs | All admin actions logged, PQ-signed, scatter-stored |
| DLP | Classification tags + rules prevent sensitive data leaving org |
| GDPR | Per-user data export, right to deletion (with legal hold exception) |
| HIPAA | BAA available, SOVEREIGN tier with HSM |
| SOC 2 | Platform-level certification (eStream) |

---

## Related Documents

- [ARCHITECTURE.md](./ARCHITECTURE.md) -- Technical architecture
- [polylabs/business/PRODUCT_FAMILY.md] -- Product specifications
