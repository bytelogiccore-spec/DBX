---
layout: default
title: 설치
parent: Node.js (dbx-native)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 1
---

# 설치

## npm에서 설치

```bash
npm install dbx-native
```

## yarn 사용

```bash
yarn add dbx-native
```

## pnpm 사용

```bash
pnpm add dbx-native
```

## 요구사항

- **Node.js**: 16 이상
- **플랫폼**: Windows x64 (현재 테스트 완료)
  - Linux x64: 계획됨
  - macOS (Intel/Apple Silicon): 계획됨

## 설치 확인

```javascript
const { Database } = require('dbx-native');

const db = Database.openInMemory();
console.log('DBX Native loaded successfully!');
db.close();
```

## TypeScript 설정

```bash
npm install --save-dev @types/node
```

`tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "esModuleInterop": true,
    "strict": true
  }
}
```

## 문제 해결

### 모듈을 찾을 수 없음

**원인**: 네이티브 바인딩 로드 실패

**해결**:
```bash
# node_modules 재설치
rm -rf node_modules package-lock.json
npm install
```

### Windows에서 빌드 도구 오류

**원인**: Visual C++ Build Tools 누락

**해결**:
1. [Build Tools for Visual Studio](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) 다운로드
2. "Desktop development with C++" 워크로드 선택
3. 설치 후 재부팅

### Node.js 버전 확인

```bash
node --version  # v16.0.0 이상 필요
```
