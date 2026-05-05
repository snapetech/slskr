import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import {
  LayoutDashboard,
  Key,
  Webhook,
  Database,
  Activity,
  Settings,
  LogOut
} from 'lucide-react';

export default function Sidebar() {
  const location = useLocation();

  const links = [
    { path: '/', label: 'Dashboard', icon: LayoutDashboard },
    { path: '/api-keys', label: 'API Keys', icon: Key },
    { path: '/webhooks', label: 'Webhooks', icon: Webhook },
    { path: '/database', label: 'Database', icon: Database },
    { path: '/monitoring', label: 'Monitoring', icon: Activity },
    { path: '/configuration', label: 'Configuration', icon: Settings },
  ];

  return (
    <div className="w-64 bg-gray-900 text-white shadow-lg">
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
                ? 'bg-blue-600 text-white'
                : 'text-gray-300 hover:bg-gray-800 hover:text-white'
            }`}
          >
            <Icon className="w-5 h-5 mr-3" />
            {label}
          </Link>
        ))}
      </nav>

      <div className="absolute bottom-0 w-64 p-6 border-t border-gray-700">
        <button className="flex items-center w-full text-gray-300 hover:text-white transition-colors">
          <LogOut className="w-5 h-5 mr-3" />
          Logout
        </button>
      </div>
    </div>
  );
}
