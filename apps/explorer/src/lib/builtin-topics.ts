/**
 * Descriptions for the DDS RTPS builtin discovery topics.
 * Every DomainParticipant automatically has readers/writers on these topics
 * — that's how discovery itself is implemented (DDS over DDS).
 */
export const BUILTIN_TOPICS: Record<string, string> = {
  DCPSParticipant:
    "SPDP — Simple Participant Discovery Protocol.\n" +
    "Every participant periodically writes a self-announcement (its GUID, " +
    "locators, vendor, lease duration) to this topic over UDP multicast. " +
    "Reading this topic is how peers find each other in the first place.",

  DCPSPublication:
    "SEDP — Endpoint discovery for DataWriters.\n" +
    "Once two participants know each other (via SPDP), they exchange the " +
    "list of their DataWriters here, with topic name, type and QoS. This " +
    "is how a remote DataReader learns about a writer it can match.",

  DCPSSubscription:
    "SEDP — Endpoint discovery for DataReaders.\n" +
    "Symmetric counterpart of DCPSPublication: each participant publishes " +
    "its DataReaders here so writers know whom to send samples to and " +
    "whether QoS matches.",

  DCPSTopic:
    "SEDP — Topic-level metadata (optional in the spec).\n" +
    "Carries topic name, type, and topic-level QoS. Many implementations " +
    "barely use it because publication/subscription announcements already " +
    "carry the same info per endpoint.",

  DCPSParticipantMessage:
    "Liveliness — automatic liveliness assertions.\n" +
    "When a participant has writers with AUTOMATIC or " +
    "MANUAL_BY_PARTICIPANT liveliness QoS, it periodically writes here to " +
    "prove it's still alive. Peers listening can detect a dead participant " +
    "before the SPDP lease times out.",
};

export function isBuiltinTopic(name: string): boolean {
  return name in BUILTIN_TOPICS;
}

export function builtinTopicDescription(name: string): string | null {
  return BUILTIN_TOPICS[name] ?? null;
}
