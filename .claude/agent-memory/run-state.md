# Autonomous run-state (template)

Checked-in template. Live values may be machine-local; do not put secrets here.

## Crons (local machine timezone)

| Name | Schedule | Skill | Cron id |
|------|----------|-------|---------|
| heartbeat | every 2 hours | `/heartbeat` | _(fill when registered)_ |
| dream | daily at local midnight | `/dream` | _(fill when registered)_ |

Timezone: **local machine**, not UTC unless the host is set to UTC.

## Tick log (heartbeat keeps last 5)

```
# YYYY-MM-DDTHH:MM:SS  action  detail
```

## In-flight

- worktree:
- branch:
- workflow run id:
- ticket:
