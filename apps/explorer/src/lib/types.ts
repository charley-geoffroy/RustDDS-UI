export type ParticipantDto = {
  guid: string;
  entity_name: string | null;
  vendor_id: string;
};

export type EndpointDto = {
  guid: string;
  participant_guid: string;
  topic_name: string;
  type_name: string;
  qos: Record<string, unknown>;
};

export type TopicDto = {
  name: string;
  type_name: string;
};

export type RegistrySnapshot = {
  participants: ParticipantDto[];
  topics: TopicDto[];
  writers: EndpointDto[];
  readers: EndpointDto[];
};

export type SampleDto = {
  topic: string;
  recv_ns: number;
  size: number;
  bytes_hex: string;
};

export type SampleBatchDto = {
  topic: string;
  samples: SampleDto[];
  received_since_last: number;
};
