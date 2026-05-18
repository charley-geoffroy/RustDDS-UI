/**
 * Docs section — chapter registry.
 *
 * Each chapter ships in both English and French (`slug.en.svx` and
 * `slug.fr.svx`). The active language is tracked at the page level and
 * passed down to the docs layout.
 *
 * Add a new chapter:
 *   1. Create `slug.en.svx` and `slug.fr.svx` in this folder.
 *   2. Add it to `LOADERS` below with both lang entries.
 *   3. Add a `ChapterMeta` entry to `CHAPTERS` with bilingual titles.
 */

import type { Component } from "svelte";

export type Lang = "en" | "fr";

export const LANGS: { code: Lang; label: string; flag: string }[] = [
  { code: "en", label: "English", flag: "EN" },
  { code: "fr", label: "Français", flag: "FR" },
];

export type GroupId = "discover" | "practice" | "ros" | "reference";

export type ChapterMeta = {
  slug: string;
  title: Record<Lang, string>;
  estimateMin: number;
  group: GroupId;
  status: "ready" | "comingSoon";
  comingInPhase?: "B" | "C" | "D" | "E" | "F";
};

export const GROUPS: { id: GroupId; label: Record<Lang, string> }[] = [
  { id: "discover",  label: { en: "Discover",    fr: "Découvrir"  } },
  { id: "practice",  label: { en: "Practice",    fr: "Pratique"   } },
  { id: "ros",       label: { en: "ROS 2 & ops", fr: "ROS 2 & ops"} },
  { id: "reference", label: { en: "Reference",   fr: "Référence"  } },
];

export const CHAPTERS: ChapterMeta[] = [
  // Discover
  { slug: "welcome",        estimateMin: 3,  group: "discover",  status: "ready",
    title: { en: "Welcome & tour",         fr: "Bienvenue & visite" } },
  { slug: "what-is-dds",    estimateMin: 5,  group: "discover",  status: "ready",
    title: { en: "What is DDS?",           fr: "DDS, c'est quoi ?" } },
  { slug: "dds-objects",    estimateMin: 5,  group: "discover",  status: "ready",
    title: { en: "The DDS objects",        fr: "Les objets DDS" } },
  { slug: "interactive-workflow", estimateMin: 6, group: "discover", status: "ready",
    title: { en: "🎬 Interactive: a complete pub/sub session",
             fr: "🎬 Interactif : une session pub/sub complète" } },
  { slug: "discovery",      estimateMin: 8,  group: "discover",  status: "ready",
    title: { en: "Discovery — how peers find each other",
             fr: "Discovery — comment les pairs se trouvent" } },

  // Practice
  { slug: "qos",            estimateMin: 12, group: "practice",  status: "ready",
    title: { en: "QoS — the matching contract",      fr: "QoS — le contrat de matching" } },
  { slug: "endpoints",      estimateMin: 6,  group: "practice",  status: "ready",
    title: { en: "Endpoints, GUIDs, instances",      fr: "Endpoints, GUIDs, instances" } },
  { slug: "cdr",            estimateMin: 15, group: "practice",  status: "ready",
    title: { en: "Serialization — from hex to text", fr: "Sérialisation — du hex au texte" } },
  { slug: "builtin-topics", estimateMin: 10, group: "practice",  status: "ready",
    title: { en: "The 5 builtin topics, in detail",  fr: "Les 5 topics builtin, en détail" } },

  // ROS 2 & ops
  { slug: "ros2",            estimateMin: 6, group: "ros",       status: "ready",
    title: { en: "ROS 2 on top of DDS",    fr: "ROS 2 au-dessus de DDS" } },
  { slug: "troubleshooting", estimateMin: 8, group: "ros",       status: "ready",
    title: { en: "Troubleshooting",        fr: "Dépannage" } },

  // Reference
  { slug: "backends",        estimateMin: 4, group: "reference", status: "ready",
    title: { en: "Backends",                fr: "Backends" } },
  { slug: "glossary",        estimateMin: 3, group: "reference", status: "ready",
    title: { en: "Glossary",                fr: "Glossaire" } },
];

// Lazy import map — every `ready` chapter must have entries for every Lang.
export const LOADERS: Record<
  string,
  Record<Lang, () => Promise<{ default: Component }>>
> = {
  welcome: {
    en: () => import("./welcome.en.svx"),
    fr: () => import("./welcome.fr.svx"),
  },
  "what-is-dds": {
    en: () => import("./what-is-dds.en.svx"),
    fr: () => import("./what-is-dds.fr.svx"),
  },
  "dds-objects": {
    en: () => import("./dds-objects.en.svx"),
    fr: () => import("./dds-objects.fr.svx"),
  },
  "interactive-workflow": {
    en: () => import("./interactive-workflow.en.svx"),
    fr: () => import("./interactive-workflow.fr.svx"),
  },
  discovery: {
    en: () => import("./discovery.en.svx"),
    fr: () => import("./discovery.fr.svx"),
  },
  qos: {
    en: () => import("./qos.en.svx"),
    fr: () => import("./qos.fr.svx"),
  },
  endpoints: {
    en: () => import("./endpoints.en.svx"),
    fr: () => import("./endpoints.fr.svx"),
  },
  cdr: {
    en: () => import("./cdr.en.svx"),
    fr: () => import("./cdr.fr.svx"),
  },
  "builtin-topics": {
    en: () => import("./builtin-topics.en.svx"),
    fr: () => import("./builtin-topics.fr.svx"),
  },
  ros2: {
    en: () => import("./ros2.en.svx"),
    fr: () => import("./ros2.fr.svx"),
  },
  troubleshooting: {
    en: () => import("./troubleshooting.en.svx"),
    fr: () => import("./troubleshooting.fr.svx"),
  },
  backends: {
    en: () => import("./backends.en.svx"),
    fr: () => import("./backends.fr.svx"),
  },
  glossary: {
    en: () => import("./glossary.en.svx"),
    fr: () => import("./glossary.fr.svx"),
  },
};

export function findChapter(slug: string): ChapterMeta | null {
  return CHAPTERS.find((c) => c.slug === slug) ?? null;
}

export function defaultChapter(): ChapterMeta {
  return CHAPTERS[0];
}

export const DEFAULT_LANG: Lang = "en";
