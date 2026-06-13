import { type FC, useMemo, useState } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router';
import { Layout, Menu, type MenuProps, theme, Typography } from 'antd';
import { BookOpenIcon, HistoryIcon, SettingsIcon } from 'lucide-react';

import AppSettingsModal from '#/app/AppSettingsModal';
import { useAppTheme } from '#/app/themeContext';
import { routes } from '#/shared/routes';

import styles from './RootLayout.module.scss';

const { Content, Header, Sider } = Layout;

const pageTitles = {
  [routes.dictionary]: 'Словарь',
  [routes.history]: 'История',
} as const;

const RootLayout: FC = () => {
  const [isSettingsModalOpen, setIsSettingsModalOpen] = useState(false);
  const { isDarkMode } = useAppTheme();
  const { token } = theme.useToken();
  const location = useLocation();
  const navigate = useNavigate();

  const currentPageKey = location.pathname.startsWith(routes.dictionary)
    ? routes.dictionary
    : routes.history;
  const currentPageTitle = pageTitles[currentPageKey];

  const menuItems = useMemo<MenuProps['items']>(
    () => [
      {
        icon: <HistoryIcon size={18} strokeWidth={2} />,
        key: routes.history,
        label: 'История',
      },
      {
        icon: <BookOpenIcon size={18} strokeWidth={2} />,
        key: routes.dictionary,
        label: 'Словарь',
      },
      {
        icon: <SettingsIcon size={18} strokeWidth={2} />,
        key: 'settings',
        label: 'Настройки',
      },
    ],
    [],
  );

  const handleMenuClick: MenuProps['onClick'] = ({ key }) => {
    if (key === 'settings') {
      setIsSettingsModalOpen(true);
      return;
    }

    void navigate(key);
  };

  const closeSettingsModal = () => {
    setIsSettingsModalOpen(false);
  };

  return (
    <Layout className={styles.root}>
      <Sider className={styles.sider} theme={isDarkMode ? 'dark' : 'light'} width={260}>
        <div className={styles.brand}>Transcriber</div>
        <Menu
          items={menuItems}
          mode="inline"
          onClick={handleMenuClick}
          selectedKeys={[currentPageKey]}
          theme={isDarkMode ? 'dark' : 'light'}
        />
      </Sider>

      <Layout>
        <Header
          className={styles.header}
          style={{
            background: token.colorBgContainer,
            borderBottomColor: token.colorBorderSecondary,
          }}
        >
          <Typography.Title className={styles.title} level={3}>
            {currentPageTitle}
          </Typography.Title>
        </Header>

        <Content className={styles.content}>
          <Outlet />
        </Content>
      </Layout>

      <AppSettingsModal open={isSettingsModalOpen} onClose={closeSettingsModal} />
    </Layout>
  );
};

export default RootLayout;
