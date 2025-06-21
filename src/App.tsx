import React, { useState, useEffect } from 'react';
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

interface LoadResult {
  path: string;
  config: Config;
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

  // 起動時に保存された設定ファイルのパスを読み込む
  useEffect(() => {
    const loadSavedConfig = async () => {
      try {
        const savedConfigPath = await invoke<string | null>('load_last_config_path');
        if (savedConfigPath) {
          setConfigPath(savedConfigPath);
          // 保存されたパスが存在する場合は自動的に設定を読み込む
          await loadConfigFromPath(savedConfigPath);
        }
      } catch (error) {
        console.error('保存された設定の読み込みに失敗しました:', error);
      }
    };
    loadSavedConfig();
  }, []);

  const loadConfigFromPath = async (path: string) => {
    try {
      setStatus({ message: '設定ファイルを読み込み中...', type: 'loading' });
      const config = await invoke<Config>('load_config', { configPath: path });
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

  const selectConfigFile = async () => {
    try {
      setStatus({ message: 'ファイルを選択中...', type: 'loading' });
      const result = await invoke<LoadResult>('select_file');
      setConfigPath(result.path);
      setCurrentConfig(result.config);
      setStatus({ 
        message: `設定を読み込みました (${result.config.rules.length}個のルール)`, 
        type: 'success' 
      });
      setResults([]);
    } catch (error) {
      setStatus({ message: `ファイル選択または設定の読み込みに失敗しました: ${error}`, type: 'error' });
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
            placeholder="右のボタンから設定ファイルを選択"
            style={{ flex: 1, minWidth: 0 }}
            readOnly
          />
          <button type="button" className="btn-secondary" onClick={selectConfigFile} style={{ whiteSpace: 'nowrap' }}>
            📁 選択して読み込み
          </button>
        </div>
      </div>

      <div className="button-group" style={{ display: 'flex', justifyContent: 'center' }}>
        <button type="button" className="btn-success" onClick={organizeFiles}>
          ファイル整理実行
        </button>
      </div>

      {status.type && (
        <div className={`status ${status.type}`}>
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