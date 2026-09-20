import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PluginResponseCommitNotice } from "../PluginResponseCommitNotice";

describe("public response commit capability notice", () => {
  it("explains complete waiting and in-flight requirements independent of plugin identity", () => {
    const { rerender } = render(
      <PluginResponseCommitNotice
        hooks={[{ name: "gateway.response.beforeCommit" }]}
        capabilities={["gateway.hooks", "gateway.provider.switch"]}
      />
    );
    expect(screen.getByText("完整响应校验")).toBeInTheDocument();
    expect(screen.getByText(/首字等待会更长/)).toBeInTheDocument();
    expect(screen.getByText(/插件申请换家时/)).toBeInTheDocument();
    expect(screen.getByText(/可安全重放时/)).toBeInTheDocument();
    expect(screen.getByText(/其他会话不受该拒绝影响/)).toBeInTheDocument();
    expect(screen.getByText(/在途请求仍须完成已确定的校验/)).toBeInTheDocument();
    rerender(<PluginResponseCommitNotice hooks={[{ name: "gateway.response.beforeCommit" }]} />);
    expect(screen.queryByText(/下一家/)).not.toBeInTheDocument();
    rerender(
      <PluginResponseCommitNotice
        hooks={[{ name: "gateway.response.after" }]}
        capabilities={["gateway.provider.switch"]}
      />
    );
    expect(screen.queryByText("完整响应校验")).not.toBeInTheDocument();
  });
});
