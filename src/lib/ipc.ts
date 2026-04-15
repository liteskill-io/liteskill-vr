import { invoke } from "@tauri-apps/api/core";

import type { ItemSummary, ItemDetail, Tag, ConnectionType } from "./types";

export async function listItems(): Promise<ItemSummary[]> {
  return invoke("list_items");
}

export async function getItem(id: string): Promise<ItemDetail> {
  return invoke("get_item", { id });
}

export async function listTags(): Promise<Tag[]> {
  return invoke("list_tags");
}

export async function listConnectionTypes(): Promise<ConnectionType[]> {
  return invoke("list_connection_types");
}
