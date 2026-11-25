//! 가상의 컨테이너 터미널 시드 데이터.
//! 모든 이름은 가상이며, 고정 시드 LCG로 생성하므로 실행할 때마다 결과가 같다.
//! 이 파일의 값을 바꾸면 테스트(engine.rs)와 docs/ontology.md의 예시 수치도 함께 확인할 것.

use crate::model::*;
use chrono::{Duration, NaiveDate};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn chance(&mut self, pct: u64) -> bool {
        self.below(100) < pct
    }
}

/// 계획 기간의 시작 시각. 모든 기항 시각은 이 시각 기준 오프셋으로 정의한다.
pub fn base_time() -> Time {
    NaiveDate::from_ymd_opt(2026, 12, 1).unwrap().and_hms_opt(0, 0, 0).unwrap()
}

fn s(v: &str) -> String {
    v.to_string()
}

pub fn seed() -> Ontology {
    let mut o = Ontology::default();
    let t0 = base_time();
    let h = |hours: i64| t0 + Duration::hours(hours);

    for (id, name) in [
        ("OP-HBL", "한빛해운"),
        ("OP-BMS", "블루메리디안해운"),
        ("OP-CRX", "코랄익스프레스"),
        ("OP-NSF", "노스스타피더"),
    ] {
        o.operators.insert(s(id), Operator { id: s(id), name: s(name) });
    }

    // id, 이름, 길이(m), 수심(m), 안벽크레인 수
    for (id, name, len, depth, cranes) in [
        ("B1", "1번 선석", 370.0, 16.5, 4),
        ("B2", "2번 선석", 370.0, 16.5, 4),
        ("B3", "3번 선석 (초대형선)", 420.0, 17.5, 5),
        ("B4", "4번 선석", 300.0, 14.0, 3),
        ("B5", "5번 선석 (피더)", 220.0, 11.0, 2),
        // 비상 대체용으로 대부분 비워 둔 선석. B1~B3보다 얕다.
        ("B6", "6번 선석 (예비)", 360.0, 16.0, 3),
    ] {
        o.berths.insert(s(id), Berth { id: s(id), name: s(name), length_m: len, depth_m: depth, cranes });
    }

    // id, 선명, 선사, 전장, 흘수, TEU
    let vessels = [
        ("VSL-01", "HANBIT AURORA", "OP-HBL", 366.0, 15.5, 14000),
        ("VSL-02", "BLUE MERIDIAN", "OP-BMS", 366.0, 15.2, 13800),
        ("VSL-03", "HANBIT TITAN", "OP-HBL", 400.0, 16.5, 24000),
        ("VSL-04", "NORTHSTAR KOBE", "OP-NSF", 172.0, 9.5, 1700),
        ("VSL-05", "CORAL HORIZON", "OP-CRX", 335.0, 14.8, 10000),
        ("VSL-06", "BLUE TIDE", "OP-BMS", 300.0, 13.5, 8500),
        ("VSL-07", "HANBIT ZENITH", "OP-HBL", 400.0, 16.3, 23500),
        ("VSL-08", "NORTHSTAR NINGBO", "OP-NSF", 196.0, 10.2, 2500),
        ("VSL-09", "CORAL VOYAGER", "OP-CRX", 349.0, 15.0, 11000),
        ("VSL-10", "BLUE PIONEER", "OP-BMS", 366.0, 15.4, 14000),
        ("VSL-11", "HANBIT ORION", "OP-HBL", 400.0, 16.4, 24000),
        ("VSL-12", "NORTHSTAR OSAKA", "OP-NSF", 180.0, 9.8, 1800),
        ("VSL-13", "NORTHSTAR QINGDAO", "OP-NSF", 196.0, 10.0, 2500),
        ("VSL-14", "CORAL SWIFT", "OP-CRX", 170.0, 9.2, 1600),
        ("VSL-15", "CORAL BREEZE", "OP-CRX", 172.0, 9.4, 1700),
        ("VSL-16", "NORTHSTAR TOKYO", "OP-NSF", 180.0, 9.6, 1800),
        ("VSL-17", "BLUE HARBOR", "OP-BMS", 294.0, 13.0, 6500),
    ];
    for (id, name, op, loa, draft, teu) in vessels {
        o.vessels.insert(
            s(id),
            Vessel { id: s(id), name: s(name), imo: format!("99{}123", &id[4..]), operator_id: s(op), loa_m: loa, draft_m: draft, teu },
        );
    }

    // id, 선박, 서비스, 항차번호, 선석, ETA(h), ETD(h), 직전 기항지, 다음 기항지들
    // 선석별 간격을 촘촘하게 잡아 지연이 R2로 연쇄되도록 했다.
    let voyages: [(&str, &str, &str, &str, &str, i64, i64, &str, &[&str]); 17] = [
        ("V17", "VSL-17", "SEA 동남아", "610S", "B6", 56, 70, "다롄", &["호찌민", "싱가포르"]),
        ("V01", "VSL-01", "AEX1 아시아-유럽", "031W", "B1", 6, 26, "상하이", &["싱가포르", "로테르담"]),
        ("V05", "VSL-05", "TPX2 태평양", "112E", "B1", 28, 46, "가오슝", &["로스앤젤레스", "오클랜드"]),
        ("V09", "VSL-09", "TPX2 태평양", "113E", "B1", 50, 70, "샤먼", &["로스앤젤레스", "오클랜드"]),
        ("V02", "VSL-02", "AEX2 아시아-지중해", "207W", "B2", 2, 24, "톈진", &["포트사이드", "발렌시아"]),
        ("V06", "VSL-06", "IAX 인도-아시아", "045S", "B2", 27, 43, "칭다오", &["포트클랑", "나바셰바"]),
        ("V10", "VSL-10", "AEX1 아시아-유럽", "032W", "B2", 46, 64, "상하이", &["싱가포르", "로테르담"]),
        ("V03", "VSL-03", "AEX3 아시아-북유럽", "018W", "B3", 10, 36, "닝보", &["로테르담", "함부르크"]),
        ("V07", "VSL-07", "TPX1 태평양", "077E", "B3", 38, 60, "옌톈", &["시애틀", "밴쿠버"]),
        ("V11", "VSL-11", "AEX3 아시아-북유럽", "019W", "B3", 62, 86, "닝보", &["로테르담", "함부르크"]),
        ("V04", "VSL-04", "KJF 한일 피더", "501N", "B4", 14, 26, "광양", &["오사카", "고베"]),
        ("V08", "VSL-08", "KCF 한중 피더", "302W", "B4", 28, 38, "인천", &["닝보", "상하이"]),
        ("V12", "VSL-12", "KJF 한일 피더", "502N", "B4", 40, 52, "울산", &["고베", "오사카"]),
        ("V13", "VSL-13", "KCF 한중 피더", "303W", "B4", 54, 64, "평택", &["칭다오", "다롄"]),
        ("V14", "VSL-14", "KJF2 한일 피더", "081E", "B5", 20, 30, "광양", &["도쿄", "요코하마"]),
        ("V15", "VSL-15", "KCF2 한중 피더", "122W", "B5", 32, 42, "인천", &["상하이", "닝보"]),
        ("V16", "VSL-16", "KJF2 한일 피더", "082E", "B5", 44, 54, "울산", &["요코하마", "도쿄"]),
    ];
    for (id, vessel, service, vno, berth, eta, etd, prev, next) in voyages {
        o.voyages.insert(
            s(id),
            Voyage {
                id: s(id),
                vessel_id: s(vessel),
                service: s(service),
                voyage_no: s(vno),
                berth_id: s(berth),
                eta: h(eta),
                etd: h(etd),
                prev_port: s(prev),
                next_ports: next.iter().map(|p| s(p)).collect(),
            },
        );
    }

    // id, 이름, 용량(TEU), 리퍼 플러그, 위험물 구역 여부
    for (id, name, cap, plugs, dg) in [
        ("YB-1A", "1A 블록", 2400, 0, false),
        ("YB-1B", "1B 블록", 2400, 0, false),
        ("YB-2A", "2A 블록", 2000, 0, false),
        ("YB-2R", "2R 블록 (냉동)", 1200, 80, false),
        ("YB-3R", "3R 블록 (냉동)", 1200, 60, false),
        ("YB-DG", "DG 블록 (위험물)", 600, 0, true),
    ] {
        o.blocks.insert(s(id), YardBlock { id: s(id), name: s(name), capacity_teu: cap, reefer_plugs: plugs, dg_area: dg });
    }

    for (id, name, tier) in [
        ("SH-01", "대한전자", "KEY"),
        ("SH-02", "서울신선식품", "KEY"),
        ("SH-03", "미래화학", "STANDARD"),
        ("SH-04", "가온오토파츠", "KEY"),
        ("SH-05", "하늘어패럴", "STANDARD"),
        ("SH-06", "퍼시픽가구", "STANDARD"),
        ("SH-07", "나라제약", "KEY"),
        ("SH-08", "동방철강상사", "STANDARD"),
    ] {
        o.shippers.insert(s(id), Shipper { id: s(id), name: s(name), tier: s(tier) });
    }

    generate_containers(&mut o);
    o
}

fn generate_containers(o: &mut Ontology) {
    let mut rng = Lcg(20261201);
    let shipper_ids: Vec<String> = o.shippers.keys().cloned().collect();
    let voyages: Vec<Voyage> = o.voyages.values().cloned().collect();
    let feeder = |v: &Voyage| v.service.contains("피더");
    let prefixes = ["HBLU", "BMSU", "CRXU", "NSFU"];
    let mut serial = 100000u64;

    let mut add = |o: &mut Ontology, rng: &mut Lcg, inbound: Option<&Voyage>, outbound: Option<&Voyage>| {
        serial += 1 + rng.below(37);
        let carrier = outbound.or(inbound).unwrap();
        let prefix = match o.vessels[&carrier.vessel_id].operator_id.as_str() {
            "OP-HBL" => prefixes[0],
            "OP-BMS" => prefixes[1],
            "OP-CRX" => prefixes[2],
            _ => prefixes[3],
        };
        let id = format!("{prefix}{serial:07}");
        let mut shipper = shipper_ids[rng.below(shipper_ids.len() as u64) as usize].clone();

        let reefer = rng.chance(14);
        // 신선식품·제약 화주는 냉동 화물 비중이 높다.
        if reefer && rng.chance(60) {
            shipper = if rng.chance(50) { s("SH-02") } else { s("SH-07") };
        }
        let imdg_class = if !reefer && rng.chance(7) {
            shipper = s("SH-03");
            Some([2u8, 3, 3, 4, 5, 8, 9][rng.below(7) as usize])
        } else {
            None
        };
        let block_id = if reefer {
            if rng.chance(55) { s("YB-2R") } else { s("YB-3R") }
        } else if imdg_class.is_some() {
            s("YB-DG")
        } else {
            [s("YB-1A"), s("YB-1B"), s("YB-2A")][rng.below(3) as usize].clone()
        };
        let destination = match outbound {
            Some(v) => v.next_ports[rng.below(v.next_ports.len() as u64) as usize].clone(),
            None => s("부산 (수입)"),
        };
        o.containers.insert(
            id.clone(),
            Container {
                id,
                iso_type: if reefer { s("45R1") } else if rng.chance(70) { s("45G1") } else { s("22G1") },
                reefer,
                set_temp_c: if reefer { Some([-18.0, -20.0, 2.0, 5.0][rng.below(4) as usize]) } else { None },
                imdg_class,
                weight_t: 8.0 + (rng.below(200) as f64) / 10.0,
                shipper_id: shipper,
                inbound_voyage_id: inbound.map(|v| v.id.clone()),
                outbound_voyage_id: outbound.map(|v| v.id.clone()),
                destination,
                block_id,
            },
        );
    };

    for inbound in voyages.iter() {
        // 최소 환적 시간(R3)은 만족하되 여유가 크지 않은 출항편을 고른다.
        // 그래야 지연이 생겼을 때 일부 환적이 현실적인 비율로 끊긴다.
        let candidates: Vec<&Voyage> = voyages
            .iter()
            .filter(|out| out.id != inbound.id)
            .filter(|out| {
                let gap = (out.etd - inbound.eta).num_hours();
                gap >= rules::MCT_H && gap <= rules::MCT_H + 30
            })
            .filter(|out| feeder(inbound) != feeder(out) || !feeder(inbound))
            .collect();

        let count = if feeder(inbound) { 18 } else { 46 };
        for _ in 0..count {
            if !candidates.is_empty() && rng.chance(if feeder(inbound) { 45 } else { 60 }) {
                let out = candidates[rng.below(candidates.len() as u64) as usize];
                add(o, &mut rng, Some(inbound), Some(out));
            } else {
                add(o, &mut rng, Some(inbound), None);
            }
        }
    }
    // 육상으로 반입되어 각 항차 선적을 기다리는 수출 화물.
    for out in voyages.iter() {
        for _ in 0..(if feeder(out) { 6 } else { 14 }) {
            add(o, &mut rng, None, Some(out));
        }
    }
}
