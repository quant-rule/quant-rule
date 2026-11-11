#!/usr/bin/env bash
#
# Keeps the QuantRule project board in step with the issues.
#
# The board uses four stages: Todo, In Progress, In Review, Done. The built-in
# workflows mark an item `Done` when its issue is closed, and close the issue
# when its item is marked `Done`. This script writes the custom `State`,
# `Item type`, `Claim Type`, `Start date`, and `Closed date` fields, preserves
# the leaf status the agent set, and recomputes a container's status from its
# sub-issues.
#
# A merged pull request reports its state as MERGED rather than CLOSED, and a
# merged pull request is closed: the rule the board states, and the rule in
# AGENTS.md section 9. Both names are closed cases here, so a merged pull
# request is `Done` and `Closed` rather than left looking like work in flight.
#
# The script computes the state the board should be in and writes it, so it is
# safe to run twice: a second run leaves the board exactly as the first one
# did.
#
# It is called by .github/workflows/project-sync.yml, which supplies:
#
#   GH_TOKEN       the GitHub App's installation token
#   ORGANIZATION   the organization that owns the project
#   PROJECT_NUMBER the project's number
#   CONTENT_ID     the node id of the issue or pull request that changed
#
# It exits 0 when the board ends up in the intended state, and 1 when it could
# not be put there.

set -euo pipefail

for required in GH_TOKEN ORGANIZATION PROJECT_NUMBER CONTENT_ID; do
  if [ -z "${!required:-}" ]; then
    echo "project-sync: ${required} is not set" >&2
    exit 1
  fi
done

gql() {
  # gql <query> [name value]...
  #
  # A value is sent as a string unless it is prefixed with `#`, which sends it
  # as a number. The marker is needed because several of the board's option
  # ids are made of digits alone, and the number a client sends decides
  # whether the API accepts it: an id sent as a number is rejected by a
  # variable declared as a string.
  local query="$1"
  shift
  local args=()
  while [ "$#" -gt 0 ]; do
    local name="$1" value="$2"
    shift 2
    if [[ "$value" == \#* ]]; then
      args+=(-F "${name}=${value#\#}")
    else
      args+=(-f "${name}=${value}")
    fi
  done
  gh api graphql -F "query=${query}" "${args[@]}"
}

# --- the project's fields, looked up by name ---------------------------------

PROJECT_FIELDS="$(gql '
query($org: String!, $number: Int!) {
  organization(login: $org) {
    projectV2(number: $number) {
      id
      fields(first: 30) {
        nodes {
          __typename
          ... on ProjectV2Field { id name }
          ... on ProjectV2SingleSelectField { id name options { id name } }
        }
      }
    }
  }
}' org "$ORGANIZATION" number "#$PROJECT_NUMBER")"

PROJECT_ID="$(echo "$PROJECT_FIELDS" | jq -r '.data.organization.projectV2.id')"

field_id() {
  # The id of a field by name, of whatever type it is.
  echo "$PROJECT_FIELDS" | jq -r --arg name "$1" '
    .data.organization.projectV2.fields.nodes
    | map(select(.name == $name))
    | .[0].id'
}

option_id() {
  echo "$PROJECT_FIELDS" | jq -r --arg field "$1" --arg name "$2" '
    .data.organization.projectV2.fields.nodes
    | map(select(.__typename == "ProjectV2SingleSelectField" and .name == $field))
    | .[0].options | map(select(.name == $name)) | .[0].id'
}

STATUS_FIELD="$(field_id "Status")"
ITEM_TYPE_FIELD="$(field_id "Item type")"
STATE_FIELD="$(field_id "State")"
CLAIM_TYPE_FIELD="$(field_id "Claim Type")"
START_DATE_FIELD="$(field_id "Start date")"
CLOSED_DATE_FIELD="$(field_id "Closed date")"

STATUS_TODO="$(option_id "Status" "Todo")"
STATUS_IN_PROGRESS="$(option_id "Status" "In Progress")"
STATUS_IN_REVIEW="$(option_id "Status" "In Review")"
STATUS_DONE="$(option_id "Status" "Done")"
ITEM_TYPE_ISSUE="$(option_id "Item type" "Issue")"
ITEM_TYPE_PULL_REQUEST="$(option_id "Item type" "Pull request")"
STATE_OPEN="$(option_id "State" "Open")"
STATE_CLOSED="$(option_id "State" "Closed")"

for id in "$STATUS_FIELD" "$ITEM_TYPE_FIELD" "$STATE_FIELD" "$CLAIM_TYPE_FIELD" \
          "$START_DATE_FIELD" "$CLOSED_DATE_FIELD" \
          "$STATUS_TODO" "$STATUS_IN_PROGRESS" "$STATUS_IN_REVIEW" "$STATUS_DONE" \
          "$ITEM_TYPE_ISSUE" "$ITEM_TYPE_PULL_REQUEST" "$STATE_OPEN" "$STATE_CLOSED"; do
  if [ -z "$id" ] || [ "$id" = "null" ]; then
    echo "project-sync: the project is missing a field or option this script writes" >&2
    exit 1
  fi
done

# --- the board ----------------------------------------------------------------

ITEMS_QUERY='
query($project: ID!, $after: String) {
  node(id: $project) {
    ... on ProjectV2 {
      items(first: 100, after: $after) {
        pageInfo { hasNextPage endCursor }
        nodes {
          id
          content {
            ... on Issue { id }
            ... on PullRequest { id }
          }
        }
      }
    }
  }
}'

find_item() {
  # find_item <content-node-id> -> the board item's id
  local content_id="$1"
  local cursor=""
  while :; do
    local page item_id
    if [ -z "$cursor" ]; then
      page="$(gql "$ITEMS_QUERY" project "$PROJECT_ID")"
    else
      page="$(gql "$ITEMS_QUERY" project "$PROJECT_ID" after "$cursor")"
    fi
    item_id="$(echo "$page" | jq -r --arg id "$content_id" '
      .data.node.items.nodes
      | map(select(.content != null and .content.id == $id))
      | .[0].id // empty')"
    if [ -n "$item_id" ]; then
      echo "$item_id"
      return 0
    fi
    cursor="$(echo "$page" | jq -r '.data.node.items.pageInfo.endCursor // empty')"
    if [ -z "$cursor" ] || [ "$(echo "$page" | jq -r '.data.node.items.pageInfo.hasNextPage')" != "true" ]; then
      return 1
    fi
  done
}

ensure_item() {
  # ensure_item <content-node-id> -> the board item's id, adding the item when
  # the board's own auto-add has not got there first
  local content_id="$1"
  local item_id
  if item_id="$(find_item "$content_id")"; then
    echo "$item_id"
    return 0
  fi
  gql '
mutation($project: ID!, $contentId: ID!) {
  addProjectV2ItemById(input: { projectId: $project, contentId: $contentId }) {
    item { id }
  }
}' project "$PROJECT_ID" contentId "$content_id" | jq -r '.data.addProjectV2ItemById.item.id'
}

read_item_fields() {
  # read_item_fields <item-id> -> {status, claim, start, close}
  gql '
query($id: ID!) {
  node(id: $id) {
    ... on ProjectV2Item {
      fieldValues(first: 30) {
        nodes {
          __typename
          ... on ProjectV2ItemFieldSingleSelectValue {
            name
            field { ... on ProjectV2SingleSelectField { name } }
          }
          ... on ProjectV2ItemFieldDateValue {
            date
            field { ... on ProjectV2Field { name } }
          }
        }
      }
    }
  }
}' id "$1" | jq -c '
    .data.node.fieldValues.nodes
    | {
        status: (map(select(.field.name == "Status")) | .[0].name // null),
        claim: (map(select(.field.name == "Claim Type")) | .[0].name // null),
        start: (map(select(.field.name == "Start date")) | .[0].date // null),
        close: (map(select(.field.name == "Closed date")) | .[0].date // null)
      }'
}

set_field() {
  # set_field <item-id> <field-id> <option-id>
  gql '
mutation($project: ID!, $item: ID!, $field: ID!, $option: String!) {
  updateProjectV2ItemFieldValue(input: {
    projectId: $project, itemId: $item, fieldId: $field,
    value: { singleSelectOptionId: $option }
  }) { projectV2Item { id } }
}' project "$PROJECT_ID" item "$1" field "$2" option "$3" >/dev/null
}

set_date() {
  # set_date <item-id> <field-id> <YYYY-MM-DD>
  gql '
mutation($project: ID!, $item: ID!, $field: ID!, $date: Date!) {
  updateProjectV2ItemFieldValue(input: {
    projectId: $project, itemId: $item, fieldId: $field,
    value: { date: $date }
  }) { projectV2Item { id } }
}' project "$PROJECT_ID" item "$1" field "$2" date "$3" >/dev/null
}

clear_field() {
  # clear_field <item-id> <field-id>
  gql '
mutation($project: ID!, $item: ID!, $field: ID!) {
  clearProjectV2ItemFieldValue(input: {
    projectId: $project, itemId: $item, fieldId: $field
  }) { projectV2Item { id } }
}' project "$PROJECT_ID" item "$1" field "$2" >/dev/null
}

# --- the issues ---------------------------------------------------------------

read_content() {
  # read_content <content-node-id> -> {type, number, state, parent, children}
  gql '
query($id: ID!) {
  node(id: $id) {
    __typename
    ... on Issue {
      number
      state
      closedAt
      parent { id number state }
      subIssues(first: 100) {
        totalCount
        nodes {
          number
          state
          projectItems(first: 5) {
            nodes {
              fieldValues(first: 20) {
                nodes {
                  ... on ProjectV2ItemFieldSingleSelectValue {
                    name
                    field { ... on ProjectV2SingleSelectField { name } }
                  }
                }
              }
            }
          }
        }
      }
    }
    ... on PullRequest { number state closedAt }
  }
}' id "$1" | jq -c '
    .data.node
    | {
        type: .__typename,
        number: .number,
        state: .state,
        closedAt: .closedAt,
        parent: (if .parent then {id: .parent.id, number: .parent.number, state: .parent.state} else null end),
        children: [(.subIssues.nodes // [])[] | {
          state: .state,
          status: ((.projectItems.nodes[0].fieldValues.nodes // [])
            | map(select(.field.name == "Status"))
            | .[0].name // null)
        }]
      }'
}

# The status a container carries, given its sub-issues.
container_status() {
  # container_status <children-json>
  local children="$1"
  local total done review open_in_review
  total="$(echo "$children" | jq 'length')"
  if [ "$total" -eq 0 ]; then
    echo "TODO"
    return 0
  fi
  done="$(echo "$children" | jq '[.[] | select(.state == "CLOSED" or .status == "Done")] | length')"
  if [ "$done" -eq "$total" ]; then
    echo "DONE"
    return 0
  fi
  review="$(echo "$children" | jq '[.[] | select(.status == "In Review")] | length')"
  open_in_review="$(echo "$children" | jq '[.[] | select(.state != "CLOSED" and .status != "In Review" and .status != "Done")] | length')"
  if [ "$review" -gt 0 ] && [ "$open_in_review" -eq 0 ]; then
    echo "IN_REVIEW"
  else
    echo "IN_PROGRESS"
  fi
}

status_option() {
  case "$1" in
    TODO) echo "$STATUS_TODO" ;;
    IN_PROGRESS) echo "$STATUS_IN_PROGRESS" ;;
    IN_REVIEW) echo "$STATUS_IN_REVIEW" ;;
    DONE) echo "$STATUS_DONE" ;;
    *) echo "project-sync: unknown status $1" >&2; exit 1 ;;
  esac
}

is_closed() {
  # is_closed <content-state> -> 0 when the content is closed
  #
  # An issue is OPEN or CLOSED. A pull request is OPEN, CLOSED, or MERGED,
  # and a merged pull request is closed.
  case "$1" in
    CLOSED|MERGED) return 0 ;;
    *) return 1 ;;
  esac
}

# Keep an issue's own open or closed state in step with the status its board
# item carries, so that the two never disagree. The board closes an issue when
# its item reaches `Done`, and nothing reopens it when a sub-issue comes back.
reconcile_issue_state() {
  # reconcile_issue_state <issue-node-id> <issue-state> <desired-status>
  local issue_id="$1" issue_state="$2" desired="$3"
  if [ "$desired" = "DONE" ] && [ "$issue_state" != "CLOSED" ]; then
    gql '
mutation($id: ID!) {
  closeIssue(input: { issueId: $id }) { issue { id state } }
}' id "$issue_id" >/dev/null
    echo "project-sync: closed issue ${issue_id}, because its board status is Done"
  elif { [ "$desired" = "IN_PROGRESS" ] || [ "$desired" = "IN_REVIEW" ]; } \
      && [ "$issue_state" = "CLOSED" ]; then
    gql '
mutation($id: ID!) {
  reopenIssue(input: { issueId: $id }) { issue { id state } }
}' id "$issue_id" >/dev/null
    echo "project-sync: reopened issue ${issue_id}, because its board status is active"
  fi
}

# --- the work -----------------------------------------------------------------

today_utc() {
  date -u +%F
}

main() {
  local content content_type content_state content_closed_at children parent_id
  content="$(read_content "$CONTENT_ID")"
  content_type="$(echo "$content" | jq -r '.type')"
  content_state="$(echo "$content" | jq -r '.state')"
  content_closed_at="$(echo "$content" | jq -r '.closedAt // empty')"
  children="$(echo "$content" | jq -c '.children')"
  parent_id="$(echo "$content" | jq -r '.parent.id // empty')"

  local item_id
  item_id="$(ensure_item "$CONTENT_ID")"

  # The custom fields that the board's own workflows do not write.
  if [ "$content_type" = "PullRequest" ]; then
    set_field "$item_id" "$ITEM_TYPE_FIELD" "$ITEM_TYPE_PULL_REQUEST"
  else
    set_field "$item_id" "$ITEM_TYPE_FIELD" "$ITEM_TYPE_ISSUE"
  fi
  if is_closed "$content_state"; then
    set_field "$item_id" "$STATE_FIELD" "$STATE_CLOSED"
  else
    set_field "$item_id" "$STATE_FIELD" "$STATE_OPEN"
  fi

  local fields current_status current_claim current_start current_close
  fields="$(read_item_fields "$item_id")"
  current_status="$(echo "$fields" | jq -r '.status // empty')"
  current_claim="$(echo "$fields" | jq -r '.claim // empty')"
  current_start="$(echo "$fields" | jq -r '.start // empty')"
  current_close="$(echo "$fields" | jq -r '.close // empty')"

  # The status. A container follows its sub-issues. A closed leaf is Done. An
  # open pull request is In Review. An open issue keeps the status the agent
  # set, and a new issue starts in Todo. A missing claim type keeps a new issue
  # in Todo and is visible in the Missing claim type view.
  local desired
  if [ "$(echo "$children" | jq 'length')" -gt 0 ]; then
    desired="$(container_status "$children")"
  elif is_closed "$content_state"; then
    desired="DONE"
  elif [ "$content_type" = "PullRequest" ]; then
    desired="IN_REVIEW"
  else
    case "$current_status" in
      "In Progress") desired="IN_PROGRESS" ;;
      "In Review") desired="IN_REVIEW" ;;
      *) desired="TODO" ;;
    esac
    if [ -z "$current_claim" ]; then
      desired="TODO"
    fi
  fi
  set_field "$item_id" "$STATUS_FIELD" "$(status_option "$desired")"

  # Start date: set once, on the first active issue transition.
  if [ "$content_type" = "Issue" ] \
      && { [ "$desired" = "IN_PROGRESS" ] || [ "$desired" = "IN_REVIEW" ]; } \
      && [ -z "$current_start" ]; then
    set_date "$item_id" "$START_DATE_FIELD" "$(today_utc)"
  fi

  # Close date: set on Done and cleared when the item returns to active work.
  if [ "$desired" = "DONE" ]; then
    if [ -z "$current_close" ]; then
      if [ -n "$content_closed_at" ]; then
        set_date "$item_id" "$CLOSED_DATE_FIELD" "${content_closed_at:0:10}"
      else
        set_date "$item_id" "$CLOSED_DATE_FIELD" "$(today_utc)"
      fi
    fi
  elif [ -n "$current_close" ]; then
    clear_field "$item_id" "$CLOSED_DATE_FIELD"
  fi

  # A container's issue is closed by the board when its item reaches `Done`.
  if [ "$(echo "$children" | jq 'length')" -gt 0 ]; then
    reconcile_issue_state "$CONTENT_ID" "$content_state" "$desired"
  fi

  # The parent of what changed is a container, and its status is recomputed
  # from its own sub-issues.
  if [ -n "$parent_id" ]; then
    local parent parent_children parent_state parent_item parent_desired
    parent="$(read_content "$parent_id")"
    parent_children="$(echo "$parent" | jq -c '.children')"
    parent_state="$(echo "$parent" | jq -r '.state')"
    parent_item="$(ensure_item "$parent_id")"
    parent_desired="$(container_status "$parent_children")"
    set_field "$parent_item" "$STATUS_FIELD" "$(status_option "$parent_desired")"
    reconcile_issue_state "$parent_id" "$parent_state" "$parent_desired"
  fi
}

main "$@"
