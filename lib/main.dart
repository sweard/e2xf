import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:e2xf/src/rust/frb_generated.dart';
import 'main_viewmodel.dart';

Future<void> main() async {
  await RustLib.init();
  runApp(MainApp(viewModel: MainViewModel()));
}

class MainApp extends StatefulWidget {
  const MainApp({super.key, required this.viewModel});
  final MainViewModel viewModel;

  @override
  State<MainApp> createState() => _MainAppState();
}

class _MainAppState extends State<MainApp> {
  final int _marginValue = 20;
  final double _radiusValue = 8;

  void _println(String msg) {
    if (kDebugMode) {
      print(msg);
    }
  }

  @override
  void initState() {
    widget.viewModel.init();
    super.initState();
  }

  @override
  void dispose() {
    widget.viewModel.dispose();
    super.dispose();
  }

  SizedBox _cfg() {
    _println("Rebuild cfg textfield");
    return SizedBox(
      height: 200,
      child: TextField(
        controller: widget.viewModel.cfgController,
        maxLines: null,
        expands: true,
        onChanged: (value) {
          widget.viewModel.updateDefaultCfg();
        },
        decoration: InputDecoration(
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(_radiusValue),
          ),
          hintText: '编辑配置内容',
          errorText: widget.viewModel.cfgErrTip,
        ),
      ),
    );
  }

  Row _item(String path, String hint, String btText, VoidCallback onPressed) {
    _println("Rebuild item btText: $btText");
    var text = "";
    if (path.isEmpty) {
      text = hint;
    } else {
      text = path;
    }
    var row = Row(
      mainAxisAlignment: MainAxisAlignment.start,
      children: [
        Expanded(
          child: Container(
            width: double.infinity,
            constraints: const BoxConstraints(minHeight: 50),
            padding: const EdgeInsets.only(
              left: 16.0,
              right: 16.0,
              top: 8.0,
              bottom: 8.0,
            ),
            decoration: BoxDecoration(
              color: Colors.grey[100],
              borderRadius: BorderRadius.circular(_radiusValue),
              border: Border.all(color: Colors.grey[300]!),
            ),
            child: Align(
              alignment: Alignment.centerLeft,
              child: Text(
                text,
                style: const TextStyle(
                  fontSize: 16,
                  fontWeight: FontWeight.normal,
                ),
                textAlign: TextAlign.left,
              ),
            ),
          ),
        ),
        const SizedBox(width: 20),
        ElevatedButton(
          style: ElevatedButton.styleFrom(
            minimumSize: const Size(180, 50),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(_radiusValue), // 设置圆角半径为 8
            ),
          ),
          onPressed: onPressed,
          child: Text(btText),
        ),
      ],
    );
    return row;
  }

  Expanded _logText() {
    final controller = widget.viewModel.scrollController;
    return Expanded(
      flex: 1,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.all(8.0),
        decoration: BoxDecoration(
          color: Colors.white,
          borderRadius: BorderRadius.circular(_radiusValue),
          border: Border.all(color: Colors.grey[300]!),
        ),
        child: SingleChildScrollView(
          controller: widget.viewModel.scrollController,
          child: ValueListenableBuilder(
            valueListenable: widget.viewModel.log,
            builder: (_, value, _) {
              WidgetsBinding.instance.addPostFrameCallback((_) {
                if (controller.hasClients) {
                  controller.animateTo(
                    controller.position.maxScrollExtent,
                    duration: const Duration(milliseconds: 200),
                    curve: Curves.easeOut,
                  );
                }
              });
              return Text(
                value,
                style: const TextStyle(
                  fontSize: 14,
                  fontFamily: 'monospace', // 等宽字体，适合日志
                  fontWeight: FontWeight.normal,
                ),
                textAlign: TextAlign.left,
              );
            },
          ),
        ),
      ),
    );
  }

  SizedBox _marginTop(int value) {
    return SizedBox(height: value.toDouble());
  }

  Row _update() {
    return Row(
      mainAxisAlignment: MainAxisAlignment.start,
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Expanded(
          child: ValueListenableBuilder<bool>(
            valueListenable: widget.viewModel.isLoading,
            builder: (context, isLoading, child) {
              return ElevatedButton(
                style: ElevatedButton.styleFrom(
                  minimumSize: Size(double.infinity, 50),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(_radiusValue),
                  ),
                ),
                onPressed: isLoading
                    ? null
                    : () {
                        widget.viewModel.update();
                      },
                child: isLoading
                    ? Row(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          SizedBox(
                            width: 20,
                            height: 20,
                            child: CircularProgressIndicator(
                              strokeWidth: 2,
                              valueColor: AlwaysStoppedAnimation<Color>(
                                Colors.white,
                              ),
                            ),
                          ),
                          SizedBox(width: 12),
                          Text('转换中...'),
                        ],
                      )
                    : Text('开始转换'),
              );
            },
          ),
        ),
        ValueListenableBuilder(
          valueListenable: widget.viewModel.useQuickUpdate,
          builder: (_, useQuickUpdate, _) {
            return Row(
              children: [
                SizedBox(width: 16),
                Text('快速转换'),
                Checkbox(
                  value: useQuickUpdate,
                  onChanged: (value) {
                    if (value != null) {
                      widget.viewModel.useQuickUpdate.value = value;
                    }
                  },
                ),
              ],
            );
          },
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        body: Padding(
          padding: const EdgeInsets.all(16.0),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.start,
            children: [
              ValueListenableBuilder(
                valueListenable: widget.viewModel.cfgController,
                builder: (context, value, child) {
                  return _cfg();
                },
              ),
              _marginTop(_marginValue),
              ValueListenableBuilder(
                valueListenable: widget.viewModel.selectedExcelPath,
                builder: (context, value, child) {
                  return _item(
                    value,
                    'Excel文件未选择',
                    '选择Excel文件',
                    () => widget.viewModel.selectExcelFile(),
                  );
                },
              ),
              _marginTop(_marginValue),
              ValueListenableBuilder(
                valueListenable: widget.viewModel.selectedXmlFolderPath,
                builder: (context, value, child) {
                  return _item(
                    value,
                    '模块文件夹未选择',
                    '选择模块文件夹',
                    () => widget.viewModel.selectFolder(),
                  );
                },
              ),

              _marginTop(_marginValue),
              _update(),
              _marginTop(40),
              // 日志输出
              _logText(),
            ],
          ),
        ),
      ),
    );
  }
}
