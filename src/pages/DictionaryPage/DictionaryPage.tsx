import { Card, Empty } from 'antd';
import type { FC } from 'react';

const DictionaryPage: FC = () => {
  return (
    <Card title="Словарь">
      <Empty description="Словарь пока пуст" />
    </Card>
  );
};

export default DictionaryPage;
