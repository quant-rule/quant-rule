#!/usr/bin/env bash
#
# Keeps the QuantRule project board in step with the issues.
#
# The board's built-in workflows write the `Status` field: an item added takes
# `Todo`, an issue closed takes `Done`, a pull request merged takes `Done`, and
# an item marked `Done` has its issue closed. None of them writes the custom
# `State` and `Item type` fields, and none of them recomputes a container's
# status from its sub-issues. This script does those three things, so that a
# parent never reads as done while its children are open, whoever made the
# change.
#
# The script computes the state the board should be in and writes only what
# differs, so it is safe to run twice.
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
  # A whole number is sent as a number, so that an Int variable is accepted.
  local query="$1"
  shift
  local args=()
  while [ "$#" -gt 0 ]; do
    local name="$1" value="$2"
    shift 2
    if [[ "$value" =~ ^-?[0-9]+$ ]]; then
      args+=(-F "${name}=${value}")
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
}' org "$ORGANIZATION" number "$PROJECT_NUMBER")"

PROJECT_ID="$(echo "$PROJECT_FIELDS" | jq -r '.data.organization.projectV2.id')"

field_id() {
  echo "$PROJECT_FIELDS" | jq -r --arg name "$1" '
    .data.organization.projectV2.fields.nodes
    | map(select(.__typename == "ProjectV2SingleSelectField" and .name == $name))
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

STATUS_TODO="$(option_id "Status" "Todo")"
STATUS_IN_PROGRESS="$(option_id "Status" "In Progress")"
STATUS_DONE="$(option_id "Status" "Done")"
ITEM_TYPE_ISSUE="$(option_id "Item type" "Issue")"
ITEM_TYPE_PULL_REQUEST="$(option_id "Item type" "Pull request")"
STATE_OPEN="$(option_id "State" "Open")"
STATE_CLOSED="$(option_id "State" "Closed")"

for id in "$STATUS_FIELD" "$ITEM_TYPE_FIELD" "$STATE_FIELD" \
          "$STATUS_TODO" "$STATUS_IN_PROGRESS" "$STATUS_DONE" \
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
      parent { id number state }
      subIssues(first: 100) { totalCount nodes { number state } }
    }
    ... on PullRequest { number state }
  }
}' id "$1" | jq -c '
    .data.node
    | {
        type: .__typename,
        number: .number,
        state: .state,
        parent: (if .parent then {id: .parent.id, number: .parent.number, state: .parent.state} else null end),
        children: [(.subIssues.nodes // [])[].state]
      }'
}

# The status a container carries, given the states of its sub-issues.
container_status() {
  # container_status <children-json>
  local children="$1"
  local total closed
  total="$(echo "$children" | jq 'length')"
  if [ "$total" -eq 0 ]; then
    echo "TODO"
    return 0
  fi
  closed="$(echo "$children" | jq '[.[] | select(. == "CLOSED")] | length')"
  if [ "$closed" -eq "$total" ]; then
    echo "DONE"
  else
    echo "IN_PROGRESS"
  fi
}

status_option() {
  case "$1" in
    TODO) echo "$STATUS_TODO" ;;
    IN_PROGRESS) echo "$STATUS_IN_PROGRESS" ;;
    DONE) echo "$STATUS_DONE" ;;
    *) echo "project-sync: unknown status $1" >&2; exit 1 ;;
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
  elif [ "$desired" = "IN_PROGRESS" ] && [ "$issue_state" = "CLOSED" ]; then
    gql '
mutation($id: ID!) {
  reopenIssue(input: { issueId: $id }) { issue { id state } }
}' id "$issue_id" >/dev/null
    echo "project-sync: reopened issue ${issue_id}, because its board status is In Progress"
  fi
}

# --- the work -----------------------------------------------------------------

main() {
  local content content_type content_state children parent_id
  content="$(read_content "$CONTENT_ID")"
  content_type="$(echo "$content" | jq -r '.type')"
  content_state="$(echo "$content" | jq -r '.state')"
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
  if [ "$content_state" = "CLOSED" ]; then
    set_field "$item_id" "$STATE_FIELD" "$STATE_CLOSED"
  else
    set_field "$item_id" "$STATE_FIELD" "$STATE_OPEN"
  fi

  # The status. A container follows its sub-issues; a leaf follows its own state.
  local desired
  if [ "$(echo "$children" | jq 'length')" -gt 0 ]; then
    desired="$(container_status "$children")"
  elif [ "$content_state" = "CLOSED" ]; then
    desired="DONE"
  else
    desired="TODO"
  fi
  set_field "$item_id" "$STATUS_FIELD" "$(status_option "$desired")"

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
