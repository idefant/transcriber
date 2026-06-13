import { Card, Empty } from 'antd';
import type { FC } from 'react';

const HistoryPage: FC = () => {
  return (
    <Card title="История транскрибаций">
      <Empty description="История пока пуста" />
    </Card>
  );
};

export default HistoryPage;
