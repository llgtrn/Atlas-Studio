# Cross-CAP Contract Matrix

| Edge | Producer | Consumer | Owner | Flow | Operation |
| --- | --- | --- | --- | --- | --- |
| XCAP-STORE-HELPDESK | CAP17 | CAP01 | CAP01 | FLOW-CROSS-STORE-HELPDESK-001 | helpdesk.ticket.create |
| XCAP-STORE-CRM | CAP17 | CAP05 | CAP05 | FLOW-CROSS-STORE-CRM-001 | crm.customer.read |
| XCAP-EMPLOY-DISCLOSE | CAP16 | CAP18 | CAP18 | FLOW-CROSS-EMPLOY-DISCLOSE-001 | effective_data_view.build |
| XCAP-WEB2APP-EMPLOY | CAP15 | CAP16 | CAP16 | FLOW-CROSS-WEB2APP-EMPLOY-001 | webapp.invoke |
| XCAP-INBOUND-NOTIFY | CAP13 | CAP07 | CAP13 | FLOW-CROSS-INBOUND-NOTIFY-001 | inbound.to_notification |
| XCAP-VAULT-BLACKBOX | CAP12 | CAP18 | CAP18 | FLOW-CAP18-SECRET-USE-001 | secret.use |
| XCAP-HD-BLACKBOX | CAP01 | CAP18 | CAP18 | FLOW-CAP18-HELPDESK-001 | message.send |
