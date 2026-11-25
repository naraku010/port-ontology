// 백엔드가 돌려주는 식별자(영문 코드)를 화면용 한글로 바꾸는 표.
// 식별자 자체는 MCP/LLM과 공유하므로 바꾸지 않고, 표시할 때만 변환한다.

export const ENTITY_TYPE: Record<string, string> = {
  Operator: "선사",
  Vessel: "선박",
  Berth: "선석",
  Voyage: "항차",
  YardBlock: "장치장 블록",
  Shipper: "화주",
  Container: "컨테이너",
  Event: "이벤트",
  ConnectionGroup: "환적 묶음",
};

export const RELATION: Record<string, string> = {
  OPERATED_BY: "운항사",
  PERFORMED_BY: "투입 선박",
  BERTHS_AT: "접안 선석",
  DISCHARGED_FROM: "양하 항차",
  LOADED_ONTO: "선적 항차",
  STORED_IN: "장치 위치",
  OWNED_BY: "소유 화주",
};

export const PROPERTY: Record<string, string> = {
  name: "이름",
  imo: "IMO 번호",
  operator_id: "선사",
  loa_m: "전장(m)",
  draft_m: "흘수(m)",
  teu: "적재량(TEU)",
  length_m: "길이(m)",
  depth_m: "수심(m)",
  cranes: "안벽크레인",
  vessel_id: "선박",
  service: "서비스",
  voyage_no: "항차번호",
  berth_id: "선석",
  eta: "입항 예정(ETA)",
  etd: "출항 예정(ETD)",
  prev_port: "직전 기항지",
  next_ports: "다음 기항지",
  capacity_teu: "용량(TEU)",
  reefer_plugs: "리퍼 플러그",
  dg_area: "위험물 구역",
  tier: "등급",
  iso_type: "ISO 타입",
  reefer: "냉동",
  set_temp_c: "설정 온도(°C)",
  imdg_class: "위험물 등급",
  weight_t: "중량(t)",
  shipper_id: "화주",
  inbound_voyage_id: "입항 항차",
  outbound_voyage_id: "출항 항차",
  destination: "목적지",
  block_id: "장치 블록",
  kind: "종류",
};

export const VALUE: Record<string, string> = {
  KEY: "핵심",
  STANDARD: "일반",
  IMPORT: "수입",
  EXPORT: "수출",
  TRANSSHIPMENT: "환적",
  HIGH: "높음",
  NORMAL: "보통",
  true: "예",
  false: "아니오",
};

export const label = (table: Record<string, string>, key: string) => table[key] ?? key;
