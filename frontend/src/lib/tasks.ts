import type { Task } from "./types";
export const taskCatalog: {
  type: Task["task_type"];
  platform: string;
  label: string;
  placeholder: string;
  evidence: string;
}[] = [
  {
    type: "X_FOLLOW",
    platform: "X",
    label: "Follow an X account",
    placeholder: "https://x.com/yourproject",
    evidence:
      "Provide your X username and a screenshot showing that you follow the account.",
  },
  {
    type: "X_REPOST",
    platform: "X",
    label: "Repost on X",
    placeholder: "https://x.com/yourproject/status/123",
    evidence: "Provide a link to your repost and your X username.",
  },
  {
    type: "X_LIKE",
    platform: "X",
    label: "Like an X post",
    placeholder: "https://x.com/yourproject/status/123",
    evidence: "Provide your X username and a screenshot of the liked post.",
  },
  {
    type: "X_COMMENT",
    platform: "X",
    label: "Comment on X",
    placeholder: "https://x.com/yourproject/status/123",
    evidence: "Provide a link to your comment.",
  },
  {
    type: "X_TAG",
    platform: "X",
    label: "Tag friends on X",
    placeholder: "https://x.com/yourproject/status/123",
    evidence: "Provide a link to your comment tagging the requested accounts.",
  },
  {
    type: "DISCORD_JOIN",
    platform: "Discord",
    label: "Join a Discord server",
    placeholder: "https://discord.gg/yourinvite",
    evidence:
      "Provide your Discord username and a screenshot showing your server membership.",
  },
  {
    type: "DISCORD_MESSAGE",
    platform: "Discord",
    label: "Post in a Discord channel",
    placeholder: "https://discord.com/channels/server/channel",
    evidence: "Provide a link to your message and your Discord username.",
  },
  {
    type: "TELEGRAM_CHANNEL_JOIN",
    platform: "Telegram",
    label: "Join a Telegram channel",
    placeholder: "https://t.me/yourchannel",
    evidence:
      "Provide your Telegram username and a screenshot showing your channel membership.",
  },
  {
    type: "TELEGRAM_GROUP_JOIN",
    platform: "Telegram",
    label: "Join a Telegram group",
    placeholder: "https://t.me/yourgroup",
    evidence:
      "Provide your Telegram username and a screenshot showing your group membership.",
  },
  {
    type: "TELEGRAM_MESSAGE",
    platform: "Telegram",
    label: "Post in a Telegram group",
    placeholder: "https://t.me/yourgroup",
    evidence:
      "Provide a message link or screenshot and your Telegram username.",
  },
  {
    type: "YOUTUBE_SUBSCRIBE",
    platform: "YouTube",
    label: "Subscribe on YouTube",
    placeholder: "https://www.youtube.com/@yourchannel",
    evidence:
      "Provide a screenshot showing your subscription and your YouTube handle.",
  },
  {
    type: "YOUTUBE_COMMENT",
    platform: "YouTube",
    label: "Comment on YouTube",
    placeholder: "https://www.youtube.com/watch?v=video",
    evidence: "Provide your comment link or a screenshot showing your comment.",
  },
  {
    type: "TIKTOK_FOLLOW",
    platform: "TikTok",
    label: "Follow on TikTok",
    placeholder: "https://www.tiktok.com/@yourproject",
    evidence:
      "Provide your TikTok username and a screenshot showing that you follow the account.",
  },
  {
    type: "TIKTOK_COMMENT",
    platform: "TikTok",
    label: "Comment on TikTok",
    placeholder: "https://www.tiktok.com/@yourproject/video/123",
    evidence: "Provide a screenshot or link showing your comment and username.",
  },
  {
    type: "INSTAGRAM_FOLLOW",
    platform: "Instagram",
    label: "Follow on Instagram",
    placeholder: "https://www.instagram.com/yourproject/",
    evidence:
      "Provide your Instagram username and a screenshot showing that you follow the account.",
  },
  {
    type: "INSTAGRAM_COMMENT",
    platform: "Instagram",
    label: "Comment on Instagram",
    placeholder: "https://www.instagram.com/p/post/",
    evidence: "Provide a screenshot or link showing your comment and username.",
  },
  {
    type: "WEBSITE_VISIT",
    platform: "Website",
    label: "Visit a website",
    placeholder: "https://yourproject.com",
    evidence:
      "Describe what participants should explore and the evidence they must submit.",
  },
  {
    type: "CUSTOM",
    platform: "Custom",
    label: "Custom task",
    placeholder: "https://yourproject.com",
    evidence:
      "Explain the action participants must complete and the evidence you expect.",
  },
];
export const taskPlatforms = [...new Set(taskCatalog.map((t) => t.platform))];
export const taskInfo = (type: Task["task_type"]) =>
  taskCatalog.find((t) => t.type === type)!;
