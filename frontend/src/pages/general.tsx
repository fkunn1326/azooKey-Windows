import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { RefreshCcw, FileChartColumn } from "lucide-react";

export const General = () => {
    return (
        <div className="space-y-8">
            <section className="space-y-2">
                <h1 className="text-sm font-bold text-foreground">バージョンと更新プログラム</h1>
                <div className="flex items-center space-x-4 rounded-md border p-4">
                    <RefreshCcw />
                    <div className="flex-1 space-y-1">
                        <p className="text-sm font-medium leading-none">
                            v0.0.0
                        </p>
                        <p className="text-xs text-muted-foreground">
                            最終確認: YYYY/MM/DD
                        </p>
                    </div>
                    <Button  variant="secondary">
                        更新プログラムを確認する
                    </Button>
                </div>
            </section>
            <section className="space-y-2">
                <h1 className="text-sm font-bold text-foreground">診断とフィードバック</h1>
                <div className="flex items-center space-x-4 rounded-md border p-4">
                    <FileChartColumn />
                    <div className="flex-1 space-y-1">
                        <p className="text-sm font-medium leading-none">
                            診断データ
                        </p>
                        <p className="text-xs text-muted-foreground">
                            診断データを保存し、バグの修正に役立てます
                        </p>
                    </div>
                    <Switch />
                </div>
            </section>
        </div>
    )
}