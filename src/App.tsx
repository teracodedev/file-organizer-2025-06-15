import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './App.css';

interface OrganizeRule {
  name: string;
  source_folder: string;
  pattern: string;
  destination_folder: string;
}

interface Config {
  rules: OrganizeRule[];
}

type StatusType = 'success' | 'error' | 'loading' | null;

const App: React.FC = () => {
  const [configPath, setConfigPath] = useState<string>('');
  const [currentConfig, setCurrentConfig] = useState<Config | null>(null);
  const [results, setResults] = useState<string[]>([]);
  const [status, setStatus] = useState<{message: string; type: StatusType}>({
    message: '',
    type: null
  });

  const selectConfigFile = async () => {
    try {
      const path = await invoke<string>('select_file');
      setConfigPath(path);
      setStatus({ message: 'ファイルが選択されました', type: 'success' });
    } catch (error) {
      setStatus({ message: 'ファイル選択がキャンセルされました', type: 'error' });
    }
  };

  const loadConfig = async () => {
    if (!configPath) {
      setStatus({ message: '設定ファイルを選択してください', type: 'error' });
      return;
    }

    try {
      setStatus({ message: '設定ファイルを読み込み中...', type: 'loading' });
      const config = await invoke<Config>('load_config', { configPath });
      setCurrentConfig(config);
      setStatus({ 
        message: `設定を読み込みました (${config.rules.length}個のルール)`, 
        type: 'success' 
      });
      setResults([]);
    } catch (error) {
      setStatus({ 
        message: `設定ファイルの読み込みに失敗しました: ${error}`, 
        type: 'error' 
      });
      setCurrentConfig(null);
    }
  };

  const organizeFiles = async () => {
    if (!configPath) {
      setStatus({ message: '設定ファイルを選択してください', type: 'error' });
      return;
    }

    try {
      setStatus({ message: 'ファイル整理を実行中...', type: 'loading' });
      const results = await invoke<string[]>('organize_files', { configPath });
      setResults(results);
      setStatus({ message: 'ファイル整理が完了しました', type: 'success' });
    } catch (error) {
      setStatus({ 
        message: `ファイル整理に失敗しました: ${error}`, 
        type: 'error' 
      });
    }
  };

  return (
    <div className="container">
      <h1>🗂️ File Organizer</h1>
      
      <div className="form-group">
        <label htmlFor="configPath">設定ファイル (YAML):</label>
        <div className="button-group" style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
          <input
            type="text"
            id="configPath"
            value={configPath}
            onChange={(e) => setConfigPath(e.target.value)}
            placeholder="設定ファイルのパスを選択してください"
            style={{ flex: 1, minWidth: 0 }}
          />
          <button type="button" className="btn-secondary" onClick={selectConfigFile} style={{ whiteSpace: 'nowrap' }}>
            📁 選択
          </button>
        </div>
      </div>

      <div className="button-group">
        <button type="button" className="btn-primary" onClick={loadConfig}>
          ⚙️ 設定を読み込み
        </button>
        <button type="button" className="btn-success" onClick={organizeFiles}>
          🚀 ファイル整理実行
        </button>
      </div>

      {status.type && (
        <div className={`status ${status.type}`}>
          {status.type === 'loading' && <div className="loading"></div>}
          {status.message}
        </div>
      )}

      {currentConfig && (
        <div className="results">
          <h3>📋 読み込まれた設定</h3>
          {currentConfig.rules.map((rule, index) => (
            <div key={index} className="result-item">
              <strong>{rule.name}</strong><br />
              📂 {rule.source_folder} → 📁 {rule.destination_folder}<br />
              🔍 パターン: <code>{rule.pattern}</code>
            </div>
          ))}
        </div>
      )}

      {results.length > 0 && (
        <div className="results">
          <h3>📊 実行結果</h3>
          {results.map((result, index) => (
            <div key={index} className="result-item">
              {result}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default App;