import React from "react";
import { Link, useLocation } from "react-router-dom";
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

  const links = [
    { path: "/", label: "Dashboard", icon: LayoutDashboard },
    { path: "/api-keys", label: "API Keys", icon: Key },
    { path: "/webhooks", label: "Webhooks", icon: Webhook },
    { path: "/database", label: "Database", icon: Database },
    { path: "/monitoring", label: "Monitoring", icon: Activity },
    { path: "/configuration", label: "Configuration", icon: Settings },
  ];

  return (
    <div className="relative flex w-64 flex-col bg-gray-900 text-white shadow-lg">
      <div className="p-6">
        <h1 className="text-2xl font-bold">slskr</h1>
        <p className="text-gray-400 text-sm">Admin Dashboard</p>
      </div>

      <nav className="mt-8">
        {links.map(({ path, label, icon: Icon }) => (
          <Link
            key={path}
            to={path}
            className={`flex items-center px-6 py-3 transition-colors ${
              location.pathname === path
                ? "bg-blue-600 text-white"
                : "text-gray-300 hover:bg-gray-800 hover:text-white"
            }`}
          >
            <Icon className="w-5 h-5 mr-3" />
            {label}
          </Link>
        ))}
      </nav>

      <div className="absolute bottom-0 w-64 border-t border-gray-700 p-6">
        <p className="mb-3 text-xs font-semibold uppercase tracking-widest text-gray-500">
          Keep the node moving
        </p>
        <div className="mb-5 grid grid-cols-2 gap-2">
          <a
            className="flex items-center justify-center gap-2 rounded-full border border-sky-500/40 bg-sky-500/10 px-3 py-2 text-xs font-semibold text-sky-300 transition-colors hover:border-sky-300 hover:text-white focus:outline-none focus:ring-2 focus:ring-sky-400"
            href="https://www.paypal.com/donate/?business=donations%40snape.tech"
            rel="noopener noreferrer"
            target="_blank"
            title="Support slskr development with PayPal"
          >
            <HeartHandshake className="h-4 w-4" /> PayPal
          </a>
          <a
            className="flex items-center justify-center gap-2 rounded-full border border-rose-400/40 bg-rose-400/10 px-3 py-2 text-xs font-semibold text-rose-300 transition-colors hover:border-rose-200 hover:text-white focus:outline-none focus:ring-2 focus:ring-rose-300"
            href="https://ko-fi.com/snapetech"
            rel="noopener noreferrer"
            target="_blank"
            title="Support slskr development on Ko-fi"
          >
            <Coffee className="h-4 w-4" /> Ko-fi
          </a>
        </div>
        <button className="flex items-center w-full text-gray-300 hover:text-white transition-colors">
          <LogOut className="w-5 h-5 mr-3" />
          Logout
        </button>
      </div>
    </div>
  );
}
