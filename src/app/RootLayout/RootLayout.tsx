import { type FC, useMemo } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router';
import { Layout, Menu, type MenuProps, theme, Typography } from 'antd';
import { BookOpenIcon, HistoryIcon, SettingsIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import AppSettingsModal from '#/app/AppSettingsModal';
import { routes } from '#/shared/routes';

import styles from './RootLayout.module.scss';

import { useUiStore } from '#/stores';

const { Content, Header, Sider } = Layout;

const RootLayout: FC = () => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const location = useLocation();
  const navigate = useNavigate();
  const openSettings = useUiStore((s) => s.openSettings);

  const currentPageKey = location.pathname.startsWith(routes.dictionary)
    ? routes.dictionary
    : routes.history;
  const currentPageTitle =
    currentPageKey === routes.dictionary ? t('navigation.dictionary') : t('navigation.history');

  const menuItems = useMemo<MenuProps['items']>(
    () => [
      {
        icon: <HistoryIcon size={18} strokeWidth={2} />,
        key: routes.history,
        label: t('navigation.history'),
      },
      {
        icon: <BookOpenIcon size={18} strokeWidth={2} />,
        key: routes.dictionary,
        label: t('navigation.dictionary'),
      },
      {
        icon: <SettingsIcon size={18} strokeWidth={2} />,
        key: 'settings',
        label: t('navigation.settings'),
      },
    ],
    [t],
  );

  const handleMenuClick: MenuProps['onClick'] = ({ key }) => {
    if (key === 'settings') {
      openSettings();
      return;
    }

    void navigate(key);
  };

  return (
    <Layout className={styles.root}>
      <Sider className={styles.sider} theme="light" width={168}>
        <div className={styles.brand}>{t('common.productName')}</div>
        <Menu
          className={styles.menu}
          items={menuItems}
          mode="inline"
          onClick={handleMenuClick}
          selectedKeys={[currentPageKey]}
        />
      </Sider>

      <Layout className={styles.mainLayout}>
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

      <AppSettingsModal />
    </Layout>
  );
};

export default RootLayout;
