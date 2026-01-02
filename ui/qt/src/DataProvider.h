#pragma once

#include <QObject>
#include <QVector>

class DataProvider : public QObject
{
    Q_OBJECT

public:
    explicit DataProvider(QObject* parent = nullptr);
    ~DataProvider();
    void start(int interval_ms = 1000);

signals:
    void dataReady(const QVector<double>& profile, int detectionCount);

private slots:
    void refresh();

private:
    class Impl;
    Impl* d;
};
