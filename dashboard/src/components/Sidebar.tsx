import React from "react";
import { Link, useLocation } from "react-router-dom";
import { useApi } from "../context/ApiContext";
import {
  LayoutDashboard,
  Key,
  Webhook,
  Database,
  Activity,
  Settings,
  LogOut,
  HeartHandshake,
  Coffee,
} from "lucide-react";

export default function Sidebar() {
  const location = useLocation();
  const { setApiKey, setIsConnected } = useApi();

  const handleLogout = () => {
    setApiKey(null);
    setIsConnected(false);
  };

  const links = [
    { path: "/", label: "Dashboard", icon: LayoutDashboard },
    { path: "/api-keys", label: "API Keys", icon: Key },
    { path: "/webhooks", label: "Webhooks", icon: Webhook },
    { path: "/database", label: "Database", icon: Database },
    { path: "/monitoring", label: "Monitoring", icon: Activity },
    { path: "/configuration", label: "Configuration", icon: Settings },
  ];

  return (
    <div className="relative flex w-16 shrink-0 flex-col bg-gray-900 text-white shadow-lg sm:w-64">
      <div className="p-2 text-center sm:p-6 sm:text-left">
        <h1 className="hidden text-2xl font-bold sm:block">slskr</h1>
        <span aria-hidden="true" className="block text-xl font-bold sm:hidden">S</span>
        <p className="hidden text-gray-400 text-sm sm:block">Admin Dashboard</p>
      </div>

      <nav className="mt-4 sm:mt-8">
        {links.map(({ path, label, icon: Icon }) => (
          <Link
            key={path}
            to={path}
            aria-current={location.pathname === path ? "page" : undefined}
            className={`flex min-h-11 items-center justify-center px-2 py-3 transition-colors sm:justify-start sm:px-6 ${
              location.pathname === path
                ? "bg-blue-600 text-white"
                : "text-gray-300 hover:bg-gray-800 hover:text-white"
            }`}
          >
            <Icon aria-hidden="true" className="mr-0 h-5 w-5 sm:mr-3" />
            <span className="sr-only sm:not-sr-only">{label}</span>
          </Link>
        ))}
      </nav>

      <div className="absolute bottom-0 w-full border-t border-gray-700 p-2 sm:w-64 sm:p-6">
        <p className="mb-3 hidden text-xs font-semibold uppercase tracking-widest text-gray-400 sm:block">
          Keep the node moving
        </p>
        <div className="mb-5 grid grid-cols-1 gap-2 sm:grid-cols-2">
          <a
            aria-label="Support slskr development with PayPal"
            className="flex min-h-8 items-center justify-center gap-2 rounded-full border border-sky-500/40 bg-sky-500/10 px-1 py-2 text-xs font-semibold text-sky-300 transition-colors hover:border-sky-300 hover:text-white focus:outline-none focus:ring-2 focus:ring-sky-400 sm:px-3"
            href="https://www.paypal.com/donate/?business=donations%40snape.tech"
            rel="noopener noreferrer"
            target="_blank"
            title="Support slskr development with PayPal"
          >
            <HeartHandshake aria-hidden="true" className="h-4 w-4 shrink-0" />
            <span className="hidden sm:inline">PayPal</span>
          </a>
          <a
            aria-label="Support slskr development on Ko-fi"
            className="flex min-h-8 items-center justify-center gap-2 rounded-full border border-rose-400/40 bg-rose-400/10 px-1 py-2 text-xs font-semibold text-rose-300 transition-colors hover:border-rose-200 hover:text-white focus:outline-none focus:ring-2 focus:ring-rose-300 sm:px-3"
            href="https://ko-fi.com/snapetech"
            rel="noopener noreferrer"
            target="_blank"
            title="Support slskr development on Ko-fi"
          >
            <Coffee aria-hidden="true" className="h-4 w-4 shrink-0" />
            <span className="hidden sm:inline">Ko-fi</span>
          </a>
        </div>
        <button
          className="flex min-h-10 w-full items-center justify-center text-gray-300 transition-colors hover:text-white sm:justify-start"
          onClick={handleLogout}
          type="button"
        >
          <LogOut aria-hidden="true" className="mr-0 h-5 w-5 sm:mr-3" />
          <span className="sr-only sm:not-sr-only">Logout</span>
        </button>
      </div>
    </div>
  );
}
