// [INPUT]: localStorage 中的主题偏好与系统 prefers-color-scheme。
// [OUTPUT]: readTheme / applyTheme / useTheme；浅色、深色、跟随系统，切换 <html class="dark">。
// [POS]: 两个入口共用的主题状态；只存这一个偏好，不存任何线路或账号数据。
import {useEffect,useSyncExternalStore} from 'react';
export type Theme='light'|'dark'|'system';
const KEY='cc-guard-theme';
const listeners=new Set<()=>void>();
const media=typeof window==='undefined'?null:window.matchMedia('(prefers-color-scheme: dark)');
export function readTheme():Theme{
  const value=typeof localStorage==='undefined'?null:localStorage.getItem(KEY);
  return value==='light'||value==='dark'?value:'system';
}
export function applyTheme(theme:Theme=readTheme()){
  const dark=theme==='dark'||(theme==='system'&&!!media?.matches);
  document.documentElement.classList.toggle('dark',dark);
  document.documentElement.style.colorScheme=dark?'dark':'light';
}
export function setTheme(theme:Theme){
  localStorage.setItem(KEY,theme);applyTheme(theme);listeners.forEach(listener=>listener());
}
const subscribe=(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener);};};
export function useTheme():Theme{
  const theme=useSyncExternalStore(subscribe,readTheme,readTheme);
  useEffect(()=>{
    if(theme!=='system'||!media)return;
    const follow=()=>applyTheme('system');
    media.addEventListener('change',follow);return()=>media.removeEventListener('change',follow);
  },[theme]);
  return theme;
}
