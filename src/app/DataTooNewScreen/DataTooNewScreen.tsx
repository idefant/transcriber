import { type FC } from 'react';
import { Result } from 'antd';
import { useTranslation } from 'react-i18next';

import ResetAppDataButton from '#/app/ResetAppDataButton';
import WindowHeader from '#/app/WindowHeader';

import styles from './DataTooNewScreen.module.scss';

/**
 * Блокирующий экран для состояния «данные новее кода»: данные в каталоге
 * записаны более новой версией приложения, поэтому эта версия их не открывает.
 * Предлагает обновиться или сбросить данные.
 *
 * Держит собственную оконную шапку: обычный `RootLayout` здесь не монтируется,
 * а без шапки окно нельзя было бы ни закрыть, ни свернуть, ни перетащить — и
 * оно оказалось бы в ловушке, если его край ушёл за границу экрана и кнопка
 * недосягаема. В шапке — только название приложения, без заголовка раздела.
 */
const DataTooNewScreen: FC = () => {
  const { t } = useTranslation();

  return (
    <div className={styles.screen}>
      <div className={styles.header}>
        <WindowHeader title={t('common.productName')} />
      </div>

      <div className={styles.body}>
        <Result
          status="warning"
          title={t('maintenance.dataTooNew.title')}
          subTitle={t('maintenance.dataTooNew.description')}
          extra={<ResetAppDataButton />}
        />
      </div>
    </div>
  );
};

export default DataTooNewScreen;
