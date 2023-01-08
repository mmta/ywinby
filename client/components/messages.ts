export interface IMessage {
  id: string,
  created_ts: number,
  owner: string,
  recipient: string,
  system_share: string,
  verify_every_minutes: number,
  max_failed_verification: number,
  owner_last_seen: number,
  recipient_last_seen: number,
  revealed: boolean,
}
