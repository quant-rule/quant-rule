/** Whether an in-flight detail response still belongs to the selected record. */
export function selectionIsCurrent(
  at: number,
  id: string,
  generation: number,
  selected: string | null,
): boolean {
  return at === generation && selected === id;
}
