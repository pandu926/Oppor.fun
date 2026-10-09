import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { request, ApiError } from "./api";
import { demo } from "./config";
import { demoCampaigns, demoEntry } from "./demo";
import type { Campaign, CampaignPage, Entry } from "./types";
import { useApp } from "./context";
export function useCampaigns(mine = false, sort = "ending") {
  const { session } = useApp();
  return useInfiniteQuery({
    queryKey: ["campaigns", mine, sort, session?.wallet],
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam, signal }): Promise<CampaignPage> =>
      demo
        ? {
            items: demoCampaigns().filter(
              (c) => !mine || c.creator_wallet === session?.wallet,
            ),
            next_cursor: null,
          }
        : request(
            `/campaigns${mine ? "/mine" : ""}?limit=24&sort=${sort}${pageParam ? `&cursor=${encodeURIComponent(pageParam)}` : ""}`,
            { signal },
          ),
    getNextPageParam: (last) => last.next_cursor,
    enabled: !mine || !!session,
    initialData:
      demo && !mine
        ? {
            pages: [{ items: demoCampaigns(), next_cursor: null }],
            pageParams: [null],
          }
        : undefined,
  });
}
export function useCampaign(id: string) {
  return useQuery({
    queryKey: ["campaign", id],
    queryFn: ({ signal }): Promise<Campaign> => {
      if (demo) {
        const found = demoCampaigns().find((c) => c.id === id);
        if (!found)
          throw new ApiError("NOT_FOUND", "This campaign does not exist.", 404);
        return Promise.resolve(found);
      }
      return request(`/campaigns/${encodeURIComponent(id)}`, { signal });
    },
    initialData: demo ? demoCampaigns().find((c) => c.id === id) : undefined,
  });
}
export function useEntry(id: string) {
  const { session } = useApp();
  return useQuery({
    queryKey: ["entry", id, session?.wallet],
    enabled: !!session,
    queryFn: async ({ signal }) => {
      if (demo) return demoEntry(id);
      try {
        return await request<Entry>(`/campaigns/${id}/my-entry`, { signal });
      } catch (e) {
        if (e instanceof ApiError && e.status === 404) return null;
        throw e;
      }
    },
  });
}
