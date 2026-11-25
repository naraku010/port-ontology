import type { Metadata } from "next";
// Pretendard 가변 폰트를 번들에 포함한다(설치형 앱이라 오프라인에서도 보여야 함). SIL OFL 1.1.
import "pretendard/dist/web/variable/pretendardvariable.css";
import "./globals.css";

export const metadata: Metadata = {
  title: "Port Ontology",
  description: "컨테이너 터미널 운영 온톨로지와 지연 영향 분석",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="ko" className="h-full antialiased">
      <body className="h-full overflow-hidden">{children}</body>
    </html>
  );
}
