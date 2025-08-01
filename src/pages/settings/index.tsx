import { accountManager } from "@/account/manager";
import { Account } from "@/account/provider";
import AccountCard from "@/components/AccountCard/AccountCard";
import CardButton from "@/components/CardButton/CardButton";
import { DisclaimerDialog } from "@/components/disclaimer/disclamierDialog";
import LoginDialog from "@/components/LoginDialog/LoginDialog";
import DropdownCard from "@/components/settings/dropdownCard";
import SettingsCard from "@/components/settings/settingsCard";
import SliderCard from "@/components/settings/sliderCard";
import SwitchCard from "@/components/settings/switchCard";
import { useAnimatedRouter } from "@/hooks/useAnimatedRouter";
import { Lang, useI18n } from "@/i18n";
import BasePage from "@/layout/basePage";
import SettingsGroup from "@/layout/settingsGroup";
import {
    Avatar,
    Body1,
    Body1Strong,
    Button,
    Checkbox,
    CheckboxOnChangeData,
    Dialog,
    DialogActions,
    DialogBody,
    DialogSurface,
    DialogTitle,
    DialogTrigger,
    Field,
    InfoLabel,
    Input,
    Label,
    Spinner
} from "@fluentui/react-components";
import {
    CloudDatabaseRegular,
    CodeBlockRegular,
    CommentMentionRegular,
    Dismiss24Regular,
    DocumentOnePageLinkRegular,
    InfoRegular,
    EarthRegular,
    LockClosedKeyRegular,
    OpenFolderRegular,
    PeopleTeamRegular,
    PersonCircleRegular,
    PersonSwapRegular,
    ReadingListRegular,
    SelectObjectSkewDismissRegular,
    TaskListAddRegular,
    TextBulletListSquareWarningRegular,
    TimelineRegular,
    Warning20Filled
} from "@fluentui/react-icons";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { platform } from "@tauri-apps/plugin-os";
import { AnimatePresence } from "framer-motion";
import { useEffect, useRef, useState } from "react";
import unlockStyles from "./unlockTool.module.css";

let config: any = {};

export default function SettingsPage() {
    const [ready, setReady] = useState(false)
    const { t } = useI18n();
    useEffect(() => {
        invoke("app_get_config").then((res) => {
            config = res;
            setReady(true);
        })
    }, [])

    return (<BasePage title={t('settings.title')}>
        <AnimatePresence>
            <AccountSettings />
            <InternelSettings />
            {ready && <><SourceSettings />
                <ListSettings />
                <InstallSettings /></>}
            <Abouts />
            <UnlockSettings />
        </AnimatePresence>

    </BasePage>)
}

function InternelSettings() {
    const { t, setLang, langs, lang, langNames } = useI18n();
    const defaultValue = localStorage.getItem("fkWebkit") === "true";
    const [debugWindow, setDebugWindow] = useState<boolean | undefined>(config?.debug_window);
    useEffect(() => { setDebugWindow(config?.debug_window); }, [config?.debug_window]);
    return <SettingsGroup title={t('settings.general.title')}>
        <DropdownCard
            title={t('settings.general.language')}
            desc={t('settings.general.languageDesc')}
            Icon={EarthRegular}
            items={Object.values(langNames)}
            onSelect={index => setLang(Object.keys(langNames)[index] as Lang)}
            defaultValue={Object.keys(langNames).indexOf(lang)}
        />
        <SettingsCard
            Icon={PeopleTeamRegular}
            title={t('settings.general.translateTeam')}>
            <Body1Strong style={{ textWrap: "nowrap" }}>{t('translators')}</Body1Strong>
        </SettingsCard>
        <SwitchCard title={t('settings.general.reduceAnimation')} desc={t('settings.general.reduceAnimationDesc')} defaultValue={defaultValue}
            Icon={SelectObjectSkewDismissRegular}
            onChange={(e) => localStorage.setItem("fkWebkit", e.toString())} />
        <SwitchCard title={t('settings.general.debugWindow')} desc={t('settings.general.debugWindowDesc')} checked={debugWindow}
            Icon={CodeBlockRegular} onChange={(e) => { setDebugWindow(e); invoke('app_write_config', { patch: { debug_window: e } }); }} />
    </SettingsGroup>
}

function UnlockSettings() {
    const { t } = useI18n();
    const [unlockCode, setUnlockCode] = useState("");
    const snRef = useRef<HTMLInputElement>(null);
    const macRef = useRef<HTMLInputElement>(null)
    const [checked, setChecked] = useState(false);
    const [mac, setMac] = useState("");
    const [sn, setSn] = useState("");
    const handleChange = (
        ev: React.ChangeEvent<HTMLInputElement>,
        data: CheckboxOnChangeData
    ) => {
        setChecked(Boolean(data.checked));
    };
    return <SettingsGroup title={t('settings.tools.title')}>
        <Dialog>
            <DialogTrigger>
                {(triggerprop) => {
                    return <CardButton
                        icon={LockClosedKeyRegular}
                        content={t('settings.tools.unlockCode')}
                        secondaryContent={t('settings.tools.unlockCodeDesc')}
                        {...triggerprop}
                    />
                }}
            </DialogTrigger>
            <DialogSurface className={unlockStyles.dialogSurface}>
                <DialogTitle className={unlockStyles.dialogTitle}>
                    <Avatar size={48} color="light-teal" icon={<LockClosedKeyRegular fontSize={24} />} />
                    {t('settings.tools.dialogTitle')}
                    <InfoLabel style={{ fontWeight: "normal" }}
                        info={t('settings.tools.dialogUsageInfo')}
                    >
                        {t('settings.tools.dialogUsage')}
                    </InfoLabel>
                </DialogTitle>
                <DialogActions>
                    <DialogTrigger disableButtonEnhancement>
                        <Button className={unlockStyles.dialogCloseButton} appearance="subtle" icon={<Dismiss24Regular />} />
                    </DialogTrigger>
                </DialogActions>
                <DialogBody style={{ display: "flex", flexDirection: "column" }}>
                    {unlockCode && (
                        <div className={unlockStyles.unlockResult}>
                            <Body1>{t('settings.tools.result')}</Body1>
                            <div className={unlockStyles.unlockDigits}>
                                {Array.from(unlockCode).map((digit, index) => (
                                    <div key={index} className={unlockStyles.digitBox}>{digit}</div>
                                ))}
                            </div>
                        </div>
                    )}
                    <Field className={unlockStyles.fieldContainer}>
                        <Label required>{t('settings.tools.mac')}</Label>
                        <Input ref={macRef} placeholder="AA:BB:CC:DD:EE:FF" style={{ width: '100%' }} onChange={(e, data) => setMac(data.value)} />
                        <Label required style={{ marginTop: 8 }}>{t('settings.tools.sn')}</Label>
                        <Input ref={snRef} placeholder="123456/ABDJHDGSGH" style={{ width: '100%' }} onChange={(e, data) => setSn(data.value)} />
                    </Field>
                    <div className={unlockStyles.actionsContainer}>
                        <div className={unlockStyles.warningContainer}>
                            <Warning20Filled />{t('settings.tools.noticeTitle')}
                        </div>
                        <Body1>
                            {t('settings.tools.noticeBody')}
                        </Body1>
                        <div className={unlockStyles.bottomRow}>
                            <Checkbox
                                className={unlockStyles.checkbox}
                                checked={checked}
                                onChange={handleChange}
                                label={t('settings.tools.agree')}
                            />
                            <Button
                                disabled={!checked || mac.trim() === "" || sn.trim() === ""}
                                appearance="primary"
                                onClick={() => {
                                    invoke<string>("miwear_get_unlock_code", {
                                        sn: snRef.current?.value?.toUpperCase(),
                                        mac: macRef.current?.value?.toUpperCase()
                                    }).then((res) => setUnlockCode(res));
                                }}
                            >
                                {t('settings.tools.calculate')}
                            </Button>
                        </div>
                    </div>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    </SettingsGroup>
}

function AccountSettings() {
    const { t } = useI18n();
    const [open, setOpen] = useState(false)
    const [bandbbsAccount, setBandbbsAccount] = useState<Account>()
    const [bandbbsLoginLoading, setBandbbsLoginLoading] = useState(false)
    useEffect(() => {
        accountManager.refresh().then(() => accountManager.get("bandbbs")?.list())
            .then(res => setBandbbsAccount(res?.[0]))
    }, [])
    const router = useAnimatedRouter()
    return <SettingsGroup title={t('settings.account.title')}>
        {!bandbbsAccount ? <CardButton
            icon={bandbbsLoginLoading ? Spinner : PersonCircleRegular}
            content={t('settings.account.loginBBS')}
            secondaryContent={t('settings.account.loginBBSDesc')}
            onClick={() => {
                setBandbbsLoginLoading(true)
                invoke<Account>("account_login", { name: "bandbbs" })
                    .then((res) => setBandbbsAccount(res))
                    .finally(() => setBandbbsLoginLoading(false))
            }}
            disabled={bandbbsLoginLoading}
        /> :
            <AccountCard
                avatar={bandbbsAccount?.avatar}
                content={bandbbsAccount?.username}
                secondaryContent={t('settings.account.bbsAccount')}
                onClick={() => router.push("/settings/bandbbs")}
            />}
        <CardButton
            icon={PersonSwapRegular}
            content={t('settings.account.syncDevices')}
            secondaryContent={t('settings.account.syncDevicesDesc')}
            onClick={() => setOpen(true)}
        />
        <LoginDialog open={open} onClose={() => setOpen(false)} />
    </SettingsGroup>
}

function InstallSettings() {
    const { t } = useI18n();
    return <SettingsGroup title={t('settings.install.title')}>
        <SliderCard title={t('settings.install.sendInterval')} desc={t('settings.install.sendIntervalDesc')} Icon={TimelineRegular} defaultValue={config?.fragments_send_delay} min={3} max={20} unit="ms"
            onChange={(e) => {
                invoke("app_write_config", {
                    patch: {
                        fragments_send_delay: e
                    }
                });
            }}
        />
    </SettingsGroup>
}

function SourceSettings() {
    const { t } = useI18n();
    const cdnList = ["raw", "jsdelivr", "ghfast", "jsdcdn"];
    const officialCdnOnSelect = (selected: number) => {
        const cdn = cdnList[selected] ?? "jsdelivr";
        invoke("app_write_config", {
            patch: {
                official_community_provider_cdn: cdn
            }
        });
    };
    const defaultValue = cdnList.findIndex((e) => e === config?.official_community_provider_cdn);
    return <SettingsGroup title={t('settings.source.title')}>
        <DropdownCard title={t('settings.source.officialCdn')} desc={t('settings.source.officialCdnDesc')} defaultValue={defaultValue} onSelect={officialCdnOnSelect} items={cdnList} Icon={CloudDatabaseRegular} />
    </SettingsGroup>
}

function ListSettings() {
    const { t } = useI18n();
    const setValue = (value: boolean, key: string) => {
        invoke("app_write_config", {
            patch: {
                [key]: value
            }
        })
    }
    return <SettingsGroup title={t('settings.queue.title')}>
        <SwitchCard title={t('settings.queue.autoInstall')} desc={t('settings.queue.autoInstallDesc')} defaultValue={config.auto_install} Icon={TaskListAddRegular} onChange={(e) => setValue(e, 'auto_install')} />
        <SwitchCard title={t('settings.queue.dontClear')} desc={t('settings.queue.dontClearDesc')} defaultValue={config.disable_auto_clean} Icon={ReadingListRegular} onChange={(e) => setValue(e, 'disable_auto_clean')} />
    </SettingsGroup>
}

function Abouts() {
    const { t } = useI18n();
    const router = useAnimatedRouter();

    const openLogDir = () => {
        if (!(platform() === "android" || platform() === "ios")) {
            invoke("app_open_log_dir")
        }
        else {
            alert("一键开启日志文件夹功能仅支持PC端。安卓手机请打开系统原生的“文件”应用，点击左上角展开列表，找到AstroBox，进入logs文件夹即可访问日志。")
        }
    };

    return <SettingsGroup title={t('settings.about.title')}>
        <CardButton
            icon={InfoRegular}
            content={t('settings.about.aboutAstrobox')}
            secondaryContent={t('settings.about.aboutAstroboxDesc')}
            onClick={() => router.push("/settings/about")}
        />
        <DisclaimerDialog trigger={<CardButton
            icon={TextBulletListSquareWarningRegular}
            content={t('settings.about.disclaimer')}
            secondaryContent={t('settings.about.disclaimerDesc')}
        />}></DisclaimerDialog>
        <CardButton
            icon={OpenFolderRegular}
            content={t('settings.about.openlog')}
            secondaryContent={t('settings.about.openlogDesc')}
            onClick={openLogDir}
            opener
        />
        <CardButton
            icon={DocumentOnePageLinkRegular}
            content={t('settings.about.website')}
            secondaryContent={t('settings.about.websiteDesc')}
            onClick={() => openUrl("https://astrobox.online")}
            opener
        />
        <CardButton
            icon={CommentMentionRegular}
            content={t('settings.about.qq')}
            secondaryContent={t('settings.about.qqDesc')}
            onClick={() => openUrl("https://qm.qq.com/q/plkzpQK35g")}
            opener
        />
        <CardButton
            icon={CodeBlockRegular}
            content={t('settings.about.licences')}
            secondaryContent={t('settings.about.licencesDesc')}
            onClick={() => router.push("/settings/licences")}
        />
    </SettingsGroup>
}