import "./globals.css";
import type { Metadata } from "next";
export const metadata: Metadata = { title: "AgentPay", description: "Programmable payments for AI agents" };
export default function RootLayout({ children }: Readonly<{children: React.ReactNode}>) { return <html lang="en"><body>{children}</body></html>; }
