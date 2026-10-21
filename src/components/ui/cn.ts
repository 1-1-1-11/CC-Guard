// [INPUT]: clsx 条件类名与 tailwind-merge 冲突合并。
// [OUTPUT]: cn()，组件库与页面共用的类名合并。
// [POS]: src/components/ui 的基础工具。
import {clsx,type ClassValue} from 'clsx';
import {twMerge} from 'tailwind-merge';
export const cn=(...values:ClassValue[])=>twMerge(clsx(values));
