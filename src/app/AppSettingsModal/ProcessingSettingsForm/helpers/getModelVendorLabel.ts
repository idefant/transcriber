import { upperFirst } from 'lodash-es';

// Написание вендоров, которое не получается простой капитализацией префикса.
const vendorLabels: Record<string, string> = {
  anthropic: 'Anthropic',
  deepseek: 'DeepSeek',
  google: 'Google',
  'meta-llama': 'Meta',
  mistralai: 'Mistral',
  openai: 'OpenAI',
  qwen: 'Qwen',
  'z-ai': 'Z.AI',
};

/**
 * Возвращает название вендора модели по её идентификатору в OpenRouter.
 *
 * OpenRouter адресует модели как `<вендор>/<модель>`, поэтому вендор берётся из префикса. Незнакомый
 * префикс отдаётся с заглавной буквы, а идентификатор без `/` — без изменений: так модель не выпадет
 * из списка, если в каталог добавят вендора, которого нет в таблице написаний.
 *
 * @example
 * getModelVendorLabel('z-ai/glm-5.3-flash'); // 'Z.AI'
 * getModelVendorLabel('deepseek/deepseek-v4.1-flash'); // 'DeepSeek'
 */
export const getModelVendorLabel = (apiId: string): string => {
  const separatorIndex = apiId.indexOf('/');

  if (separatorIndex === -1) return apiId;

  const vendor = apiId.slice(0, separatorIndex);

  return vendorLabels[vendor] ?? upperFirst(vendor);
};
