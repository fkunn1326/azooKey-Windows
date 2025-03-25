import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import { SidebarProvider } from "@/components/ui/sidebar"
import { ThemeProvider } from "@/components/theme-provider"
import { BrowserRouter, Routes, Route } from "react-router";
import { AppSidebar } from "@/components/app-sidebar"

import { General } from "@/pages/general"
import { Appearance } from "@/pages/appearance"
import { Zenzai } from "@/pages/zenzai"
import { About } from "@/pages/about"
import { Toaster } from "@/components/ui/sonner"

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <SidebarProvider>
      <BrowserRouter>
        <AppSidebar />
        <main className="w-full p-6">
          <ThemeProvider defaultTheme="system" storageKey="vite-ui-theme">
            <Routes>
              <Route path="/" element={<General />} />
              <Route path="/appearance" element={<Appearance />} />
              <Route path="/zenzai" element={<Zenzai />} />
              <Route path="/about" element={<About />} />
            </Routes>
            <Toaster />
          </ThemeProvider>
        </main>
      </BrowserRouter>
    </SidebarProvider>
  </React.StrictMode>,
);
