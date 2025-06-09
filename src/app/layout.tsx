import "./globals.css";

import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "sh.trifledmatter - virtualized terminal",
  description: "A browser-based terminal environment featuring a virtual assembly language interpreter & UNIX command support",
  keywords: [
    "terminal",
    "assembly",
    "virtual machine",
    "stack-based",
    "interpreter",
    "programming",
    "code editor",
    "command line"
  ],
  authors: [
    {
      name: "trifledmatter",
      url: "https://www.trifledmatter.com",
    },
  ],
  creator: "TrifledMatter",
  publisher: "TrifledMatter",
  openGraph: {
    title: "sh.trifledmatter - virtualized terminal",
    description: "A browser-based terminal environment with an assembly language interpreter & unix command support",
    images: ['/og-image.png'],
    type: 'website',
  },
  twitter: {
    card: "summary_large_image",
    title: "sh.trifledmatter - virtualized terminal",
    description: "A browser-based terminal environment with an assembly language interpreter & unix command support",
    images: ['/og-image.png'],
  },
  viewport: "width=device-width, initial-scale=1, maximum-scale=1",
  formatDetection: {
    telephone: false,
  },
  themeColor: "#000000",
  colorScheme: "dark",
};


export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body
        className={`antialiased`}
      >
        {children}
      </body>
    </html>
  );
}
